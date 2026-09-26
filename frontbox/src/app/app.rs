use crate::app::app_tracer::AppTracer;
use crate::app::run_loop;
use crate::hardware::*;
use crate::operator_config::OperatorConfig;
use crate::prelude::app_message::AppMessage;
use crate::prelude::*;
use crate::provided::WatchdogSystem;
use tokio::sync::mpsc;

/// Main runnable of Frontbox
pub struct App {
  systems: Vec<SystemContainer>,
  tracers: Vec<Box<dyn AppTracer>>,
  operator_config: OperatorConfig,
  boot_config: BootConfig,
}

impl App {
  pub fn new(boot_config: BootConfig) -> Self {
    let mut operator_config = match boot_config.config_path.clone() {
      Some(path) => OperatorConfig::load_from_disk(path),
      None => OperatorConfig::default(),
    };

    // look through driver configurations and extract out operator configs
    Self::register_driver_operator_configs(&boot_config.io_network.drivers, &mut operator_config);

    Self {
      systems: Vec::new(),
      tracers: Vec::new(),
      operator_config,
      boot_config,
    }
  }

  /// Register and launch system at startup
  pub fn system(&mut self, system: impl Into<SystemContainer>) -> &mut Self {
    let sys = system.into();
    // auto-register startup systems
    for cv in sys.config_values() {
      self.operator_config.register(cv);
    }

    self.systems.push(sys);
    self
  }

  /// Register a tracer to monitor things
  pub fn tracer(&mut self, tracer: impl AppTracer + 'static) -> &mut Self {
    self.tracers.push(Box::new(tracer));
    self
  }

  /// Manually register configs
  pub fn register_configs(&mut self, configs: impl IntoConfigs) -> &mut Self {
    for cv in configs.into_configs() {
      self.operator_config.register(cv);
    }
    self
  }

  pub fn configure(mut self, config_fn: impl FnOnce(&mut Self)) -> Self {
    config_fn(&mut self);
    self
  }

  pub async fn run(mut self) {
    let app_config = AppConfig::from_boot_config(&self.boot_config);
    let (app_sender, app_receiver) = mpsc::unbounded_channel::<AppMessage>();
    self.operator_config.app_sender = Some(app_sender.clone());

    let (mut machine, hardware) = match self.boot_config.platform {
      Platform::Neuron => neuron::Neuron::boot(self.boot_config, app_sender.clone()).await,
      Platform::Virtual => todo!(),
    };

    let boot_snapshot = BootSnapshot {
      switches: hardware.switches,
      drivers: hardware.drivers,
      leds: hardware.leds,
      io_network: hardware.io_network,
      exp_network: hardware.exp_network,
      app_config,
      operator_config: self.operator_config,
    };
    machine.on_pre_run(&boot_snapshot).await;

    // These systems need to appear first because other systems expect them to be present on startup
    let bridge = MachineSystem::new(machine.sender());
    self.systems.insert(0, SystemContainer::new(bridge));
    self
      .systems
      .push(SystemContainer::new(WatchdogSystem::new()));

    // Start machine task
    tokio::spawn(async move {
      machine.run().await;
    });

    log::debug!("Starting main run loop");
    run_loop::run(
      boot_snapshot,
      self.systems,
      self.tracers,
      app_sender,
      app_receiver,
    )
    .await;
  }

  /// Scan all the drivers looking for operator configs
  fn register_driver_operator_configs(
    drivers: &[IoAddressed<DriverDefinition>],
    operator_config: &mut OperatorConfig,
  ) {
    for driver in drivers {
      if let Some(mode) = &driver.definition.mode {
        for cv in mode.generalized_config_values() {
          operator_config.register(cv);
        }
      }
    }
  }
}
