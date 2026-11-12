use crate::app::app_tracer::AppTracer;
use crate::app::run_loop;
use crate::machine::*;
use crate::metrics::MetricsDb;
use crate::operator_config::ConfigCollection;
use crate::prelude::app_message::AppMessage;
use crate::prelude::app_tracer::TracerControlEvent;
use crate::prelude::*;
use crate::provided::WatchdogSystem;
use crate::systems::routine::Routines;
use fast_protocol::BoardResetCommand;
use futures_util::FutureExt;
use std::any::{TypeId, type_name};
use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::time::Duration;
use tokio::sync::mpsc;

/// Main runnable of Frontbox
pub struct App {
  systems: Vec<SystemContainer>,
  tracers: Vec<Box<dyn AppTracer>>,
  configs: ConfigCollection,
  boot_config: BootConfig,
  routines: HashMap<TypeId, RoutineDefinition>,
  planes: Vec<AppPlane>,
}

impl App {
  pub fn new(boot_config: BootConfig) -> Self {
    let configs = match boot_config.config_path.clone() {
      Some(path) => ConfigCollection::load_from_disk(path),
      None => ConfigCollection::default(),
    };

    Self {
      systems: Vec::new(),
      tracers: Vec::new(),
      configs,
      boot_config,
      routines: HashMap::new(),
      planes: Vec::new(),
    }
  }

  /// Register and launch system at startup
  pub fn system(&mut self, system: impl Into<SystemContainer>) -> &mut Self {
    self.systems.push(system.into());
    self
  }

  /// Define the routine for `E`: the systems, in order, that take part when one is started. See [routines](crate::systems::routine)
  pub fn routine<E: Event>(&mut self, definition: RoutineDefinition) -> &mut Self {
    if self
      .routines
      .insert(TypeId::of::<E>(), definition)
      .is_some()
    {
      log::warn!(
        "Routine {} was defined twice; the last definition wins",
        type_name::<E>()
      );
    }
    self
  }

  /// Add a surface of the machine, which scene files can then name and tracers are told about. Pass the plane, or
  /// `app_plane!(planes::PLAYFIELD)` so positions on it can be copied as code.
  pub fn plane(&mut self, plane: impl Into<AppPlane>) -> &mut Self {
    self.planes.push(plane.into());
    self
  }

  /// Register a tracer to monitor things
  pub fn tracer(&mut self, tracer: impl AppTracer + 'static) -> &mut Self {
    self.tracers.push(Box::new(tracer));
    self
  }

  pub fn configure(mut self, config_fn: impl FnOnce(&mut Self)) -> Self {
    config_fn(&mut self);
    self
  }

  pub async fn run(mut self) {
    let app_config = AppConfig { planes: std::mem::take(&mut self.planes), ..AppConfig::from_boot_config(&self.boot_config) };
    let (app_sender, app_receiver) = mpsc::unbounded_channel::<AppMessage>();
    let (tracer_ctl_sender, tracer_ctl_receiver) = mpsc::unbounded_channel::<TracerControlEvent>();

    self.configs.app_sender = Some(app_sender.clone());

    if let Some(path) = self.boot_config.config_path.clone() {
      self.configs.begin_save_task(path);
    }

    // opened before tracers start, so they can read history from it
    let metrics_db = self.boot_config.metrics_path.as_ref().and_then(|path| {
      MetricsDb::open(path)
        .inspect_err(|e| log::error!("Failed to open metrics store at {}: {e}", path.display()))
        .ok()
    });

    let (app_state, machine_sender) = match self.boot_config.platform {
      Platform::Neuron { .. } => {
        let (mut machine, mut hardware) =
          neuron::Neuron::boot(self.boot_config, app_sender.clone()).await;

        app_config.planes.iter().for_each(|plane| hardware.leds.add_plane(plane.plane));
        self
          .tracers
          .iter_mut()
          .for_each(|tr| tr.init(&hardware, &app_config, tracer_ctl_sender.clone(), metrics_db.clone()));
        let state = AppState::from_hardware(hardware, self.configs, app_config);
        machine.on_pre_run(&state).await;

        let machine_sender = machine.sender();
        tokio::spawn(async move {
          machine.run().await;
        });

        (state, machine_sender)
      }
      Platform::Virtual => {
        let (mut machine, mut hardware) =
          vm::VirtualMachine::boot(self.boot_config, app_sender.clone()).await;

        app_config.planes.iter().for_each(|plane| hardware.leds.add_plane(plane.plane));
        self
          .tracers
          .iter_mut()
          .for_each(|tr| tr.init(&hardware, &app_config, tracer_ctl_sender.clone(), metrics_db.clone()));
        let state = AppState::from_hardware(hardware, self.configs, app_config);
        machine.on_pre_run(&state).await;

        let machine_sender = machine.sender();
        tokio::spawn(async move {
          machine.run().await;
        });

        (state, machine_sender)
      }
    };

    if let Some(db) = metrics_db {
      app_state.metrics.begin_store_task(db);
    }

    let crash_power_down = CrashPowerDown::new(machine_sender.clone(), &app_state);

    // These systems need to appear first because other systems expect them to be present on startup
    let bridge = SystemContainer::new(MachineSystem::new(machine_sender, app_sender.clone()));
    let machine_system_id = bridge.id();
    self.systems.insert(0, bridge);
    self
      .systems
      .push(SystemContainer::new(WatchdogSystem::new()));

    log::debug!("Starting main run loop");
    let run = run_loop::run(
      app_state,
      self.systems,
      self.tracers,
      app_sender,
      app_receiver,
      tracer_ctl_receiver,
      machine_system_id,
      Routines::new(self.routines),
    );

    // A panic in a system's handler is caught in the run loop. One that gets here came from the framework itself, so
    // nothing is left to run the game: leave the machine powered down and dark rather than frozen, then carry on
    // unwinding.
    if let Err(payload) = AssertUnwindSafe(run).catch_unwind().await {
      log::error!(
        "💥 Run loop panicked: {}. Powering down the machine.",
        run_loop::panic_message(payload.as_ref())
      );
      crash_power_down.run().await;
      std::panic::resume_unwind(payload);
    }
  }
}

/// What to send the machine if the run loop dies: expire the watchdog, which cuts power to the coils, and reset the
/// expansion boards, which blanks the LEDs. Built before the run loop takes the app state.
struct CrashPowerDown {
  machine_sender: mpsc::UnboundedSender<MachineMessage>,
  exp_addresses: Vec<u8>,
}

impl CrashPowerDown {
  /// How long the machine task gets to send the power down messages before the process goes
  const FLUSH_TIME: Duration = Duration::from_millis(500);

  fn new(machine_sender: mpsc::UnboundedSender<MachineMessage>, state: &AppState) -> Self {
    Self {
      machine_sender,
      exp_addresses: state.exp_network.iter().map(|board| board.address).collect(),
    }
  }

  async fn run(&self) {
    let _ = self.machine_sender.send(MachineMessage::WatchdogClear);
    for address in &self.exp_addresses {
      let _ = self.machine_sender.send(MachineMessage::Command {
        port: MachinePort::Exp,
        command: Box::new(BoardResetCommand::new(*address)),
        timeout: COMMAND_TIMEOUT,
      });
    }
    // the machine runs on its own task, which keeps going while this one waits
    tokio::time::sleep(Self::FLUSH_TIME).await;
  }
}
