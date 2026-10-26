use crate::app::app_tracer::AppTracer;
use crate::app::run_loop;
use crate::machine::*;
use crate::operator_config::ConfigCollection;
use crate::prelude::app_message::AppMessage;
use crate::prelude::app_tracer::TracerControlEvent;
use crate::prelude::*;
use crate::provided::WatchdogSystem;
use tokio::sync::mpsc;

/// Main runnable of Frontbox
pub struct App {
  systems: Vec<SystemContainer>,
  tracers: Vec<Box<dyn AppTracer>>,
  configs: ConfigCollection,
  boot_config: BootConfig,
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
    }
  }

  /// Register and launch system at startup
  pub fn system(&mut self, system: impl Into<SystemContainer>) -> &mut Self {
    self.systems.push(system.into());
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
    let app_config = AppConfig::from_boot_config(&self.boot_config);
    let (app_sender, app_receiver) = mpsc::unbounded_channel::<AppMessage>();
    let (tracer_ctl_sender, tracer_ctl_receiver) = mpsc::unbounded_channel::<TracerControlEvent>();
    self.configs.app_sender = Some(app_sender.clone());

    let (boot_snapshot, machine_sender) = match self.boot_config.platform {
      Platform::Neuron { .. } => {
        let (mut machine, hardware) =
          neuron::Neuron::boot(self.boot_config, app_sender.clone()).await;

        self
          .tracers
          .iter_mut()
          .for_each(|tr| tr.init(&hardware, tracer_ctl_sender.clone()));
        let snapshot = BootSnapshot::from_hardware(hardware, self.configs, app_config);
        machine.on_pre_run(&snapshot).await;

        let machine_sender = machine.sender();
        tokio::spawn(async move {
          machine.run().await;
        });

        (snapshot, machine_sender)
      }
      Platform::Virtual => {
        let (mut machine, hardware) =
          vm::VirtualMachine::boot(self.boot_config, app_sender.clone()).await;

        self
          .tracers
          .iter_mut()
          .for_each(|tr| tr.init(&hardware, tracer_ctl_sender.clone()));
        let snapshot = BootSnapshot::from_hardware(hardware, self.configs, app_config);
        machine.on_pre_run(&snapshot).await;

        let machine_sender = machine.sender();
        tokio::spawn(async move {
          machine.run().await;
        });

        (snapshot, machine_sender)
      }
    };

    // These systems need to appear first because other systems expect them to be present on startup
    let bridge = SystemContainer::new(MachineSystem::new(machine_sender, app_sender.clone()));
    let machine_system_id = bridge.id();
    self.systems.insert(0, bridge);
    self
      .systems
      .push(SystemContainer::new(WatchdogSystem::new()));

    log::debug!("Starting main run loop");
    run_loop::run(
      boot_snapshot,
      self.systems,
      self.tracers,
      app_sender,
      app_receiver,
      tracer_ctl_receiver,
      machine_system_id,
    )
    .await;
  }
}
