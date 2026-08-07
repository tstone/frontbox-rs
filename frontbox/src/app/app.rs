use std::time::Duration;

use crate::app::app_config::AppConfig;
use crate::app::app_tracer::AppTracer;
use crate::app::run_loop;
use crate::hardware::*;
use crate::machine::serial_interface::SerialInterface;
use crate::operator_config::OperatorConfig;
use crate::prelude::app_message::AppMessage;
use crate::prelude::*;
use crate::provided::WatchdogSystem;
use fast_protocol::*;
use tokio::sync::mpsc;

/// Main runnable of Frontbox
pub struct App {
  io_port: SerialInterface,
  exp_port: SerialInterface,
  operator_config: OperatorConfig,
  app_config: AppConfig,
  systems: Vec<SystemContainer>,
  hardware: Hardware,
  tracers: Vec<Box<dyn AppTracer>>,
}

impl App {
  pub async fn boot(boot_config: BootConfig) -> Self {
    let app_config = AppConfig::from_boot_config(&boot_config);
    
    let mut io_port = SerialInterface::new(boot_config.io_net_port_path)
      .await
      .expect("Failed to open IO NET port");
    log::info!("🥾 Opened IO NET port at {}", boot_config.io_net_port_path);

    let mainboard_name = App::boot_mainboard(&mut io_port).await;

    // Verify user-configuration and load firmware/board versions
    let io_network = boot_config.io_network;
    let resolved_io_network = Hardware::resolve_io_network(&mut io_port, &io_network).await;
    let platform =
      FastPlatform::from_name(&mainboard_name).expect("Unsupported mainboard platform");

    App::configure_mainboard(&mut io_port, platform).await;
    App::verify_watchdog(&mut io_port).await;
    App::configure_switches(&mut io_port, &io_network.switches).await;

    // Initialize switch context which Machine will use to maintain current state
    let initial_switch_state = Hardware::get_initial_switch_states(&mut io_port).await;
    let switch_lookup = SwitchLookup::new(io_network.switches, initial_switch_state);

    // Setup operator config
    let mut operator_config = match boot_config.config_path {
      Some(path) => OperatorConfig::load_from_disk(path),
      None => OperatorConfig::new(),
    };
    App::register_driver_operator_configs(&io_network.drivers, &mut operator_config);

    // open EXP port
    let mut exp_port = SerialInterface::new(boot_config.exp_port_path)
      .await
      .expect("Failed to open EXP port");
    log::info!("🥾 Opened EXP port at {}", boot_config.exp_port_path);

    let expansion_boards = Hardware::resolve_expansion_boards(&boot_config.exp_network.boards);
    Hardware::reset_expansion_boards(&mut exp_port, &expansion_boards).await;
    Hardware::configure_led_ports(&mut exp_port, &expansion_boards).await;

    // Insert hardware definitions into store for systems to reference
    log::debug!("Initializing Store with hardware definitions");
    let hardware = Hardware::new(
      switch_lookup,
      DriverLookup::new(io_network.drivers),
      LedLookup::new(&expansion_boards),
      resolved_io_network.boards,
      expansion_boards,
    );

    Self {
      io_port,
      exp_port,
      hardware,
      operator_config,
      app_config,
      systems: Vec::new(),
      tracers: Vec::new(),
    }
  }

  /// wait for the mainboard to be ready to respond
  async fn boot_mainboard(io_port: &mut SerialInterface) -> String {
    loop {
      match io_port
        .request(&IdCommand::new(), Duration::from_millis(500))
        .await
      {
        Ok(IdResponse::Report { mainboard_name, .. }) => {
          log::info!("🥾 Mainboard is ready");
          return mainboard_name;
        }
        _ => {
          log::debug!("Mainboard not ready, retrying...");
          tokio::time::sleep(Duration::from_millis(500)).await;
        }
      };
    }
  }

  /// Initialize the mainboard with the right firmware
  async fn configure_mainboard(io_port: &mut SerialInterface, platform: FastPlatform) {
    log::info!(
      "🥾 Configuring mainboard hardware as platform {:?}",
      platform
    );
    let _ = io_port
      .request(
        &ConfigureHardwareCommand::new(platform as u16, Some(SwitchReporting::Verbose)),
        Duration::from_millis(500),
      )
      .await;
  }

  /// Verify the watchdog is responsive. Sometimes the first few commands will fail.
  async fn verify_watchdog(io_port: &mut SerialInterface) {
    loop {
      match io_port
        .request(
          &WatchdogCommand::set(Duration::from_millis(1250)),
          Duration::from_secs(1),
        )
        .await
      {
        Ok(WatchdogResponse::Processed) => {
          log::info!("🥾 Watchdog is ready");
          break;
        }
        _ => {
          tokio::time::sleep(Duration::from_millis(500)).await;
        }
      };
    }
  }

  async fn configure_switches(
    io_port: &mut SerialInterface,
    switches: &Vec<IoAddressed<SwitchDefinition>>,
  ) {
    for switch in switches {
      if let Some(config) = &switch.definition.config {
        let reporting = if config.inverted {
          SwitchReportingMode::ReportInverted
        } else {
          SwitchReportingMode::ReportNormal
        };
        log::info!(
          "Configuring switch {} with {:?}",
          switch.definition.name,
          config
        );
        let _ = io_port
          .request(
            &ConfigureSwitchCommand::new(
              switch.id,
              reporting,
              config.debounce_close,
              config.debounce_open,
            ),
            Duration::from_millis(500),
          )
          .await;
      }
    }
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
    let (app_sender, app_receiver) = mpsc::unbounded_channel::<AppMessage>();
    self.operator_config.app_sender = Some(app_sender.clone());

    let context_base = ContextBase {
      switches: self.hardware.switches,
      drivers: self.hardware.drivers,
      leds: self.hardware.leds,
      io_network: self.hardware.io_network,
      exp_network: self.hardware.exp_network,
      app_config: self.app_config,
      operator_config: self.operator_config,
    };

    Hardware::configure_drivers(&mut self.io_port, &context_base).await;

    let mut machine: MachineImpl = MachineImpl::new(
      self.io_port,
      self.exp_port,
      app_sender.clone(),
      context_base.switches.clone(),
      context_base.io_network.clone(),
      context_base.app_config.clone(),
    );

    // These systems need to appear first because other systems expect them to be present on startup
    let bridge = Machine::new(machine.sender());
    self.systems.insert(0, SystemContainer::new(bridge));
    self.systems.push(SystemContainer::new(WatchdogSystem::new()));

    // Start machine task
    tokio::spawn(async move {
      machine.run().await;
    });

    for tracer in &mut self.tracers {
      tracer.start();
    }

    log::debug!("Starting main run loop");
    run_loop::run(context_base, self.systems, self.tracers, app_sender, app_receiver).await;
  }
}


