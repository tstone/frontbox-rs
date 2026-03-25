use std::time::Duration;

use crate::app::run_loop;
use crate::hardware::*;
use crate::machine::serial_interface::SerialInterface;
use crate::plugins::Plugin;
use crate::prelude::app_message::AppMessage;
use crate::prelude::*;
use fast_protocol::*;
use tokio::sync::mpsc;

pub struct App {
  io_port: SerialInterface,
  exp_port: SerialInterface,
  operator_config: OperatorConfig,
  app_config: AppConfig,
  systems: Vec<SystemContainer>,
  hardware: Hardware,
}

impl App {
  pub async fn boot(
    config: BootConfig,
    io_network: IoNetwork,
    expansion_boards: Vec<ExpansionBoard>,
  ) -> Self {
    let mut io_port = SerialInterface::new(config.io_net_port_path)
      .await
      .expect("Failed to open IO NET port");
    log::info!("🥾 Opened IO NET port at {}", config.io_net_port_path);

    App::boot_mainboard(&mut io_port).await;
    App::configure_hardware(&mut io_port, config.platform).await;
    App::verify_watchdog(&mut io_port).await;
    App::configure_switches(&mut io_port, &io_network.switches).await;

    // Initialize switch context which Machine will use to maintain current state
    let initial_switch_state = App::get_initial_switch_states(&mut io_port).await;

    // Configure drivers
    App::configure_drivers(&mut io_port, &io_network.drivers).await;

    // open EXP port
    let mut exp_port = SerialInterface::new(config.exp_port_path)
      .await
      .expect("Failed to open EXP port");
    log::info!("🥾 Opened EXP port at {}", config.exp_port_path);

    App::reset_expansion_boards(&mut exp_port, &expansion_boards).await;
    App::configure_led_ports(&mut exp_port, &expansion_boards).await;

    // Insert hardware definitions into store for systems to reference
    log::debug!("Initializing Store with hardware definitions");
    let hardware = Hardware::new(
      SwitchLookup::new(io_network.switches, initial_switch_state),
      DriverLookup::new(io_network.drivers),
      io_network.boards,
      expansion_boards,
    );

    Self {
      io_port,
      exp_port,
      hardware,
      operator_config: OperatorConfig::new(),
      app_config: AppConfig::default(),
      systems: Vec::new(),
    }
  }

  /// wait for the mainboard to be ready to respond
  async fn boot_mainboard(io_port: &mut SerialInterface) {
    let _ = io_port
      .request_until_match(IdCommand::new(), Duration::from_millis(500), |response| {
        if let IdResponse::Report {
          processor,
          product_number,
          firmware_version,
        } = response
        {
          log::info!(
            "🥾 Connected to mainboard {} {} with firmware: {}",
            processor,
            product_number,
            firmware_version
          );
          Some(true)
        } else {
          None
        }
      })
      .await;
  }

  async fn configure_hardware(io_port: &mut SerialInterface, platform: FastPlatform) {
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

  /// Read the hardware state of all switches at startup to initialize the switch context
  async fn get_initial_switch_states(io_port: &mut SerialInterface) -> Vec<SwitchState> {
    io_port
      .request_until_match(
        ReportSwitchesCommand::new(),
        Duration::from_millis(2000),
        |resp| {
          if let SwitchReportResponse::SwitchReport { switches } = resp {
            log::debug!("🥾 Initial switch states: {:?}", switches);
            Some(switches)
          } else {
            None
          }
        },
      )
      .await
  }

  /// Verify the watchdog is responsive. Sometimes the first few commands will fail.
  async fn verify_watchdog(io_port: &mut SerialInterface) {
    let _ = io_port.request_until_match(
      WatchdogCommand::set(Duration::from_millis(1250)),
      Duration::from_secs(1),
      |resp| match resp {
        WatchdogResponse::Processed => {
          log::info!("🥾 Watchdog is ready");
          Some(true)
        }
        _ => None,
      },
    );
  }

  async fn configure_switches(io_port: &mut SerialInterface, switches: &Vec<SwitchDefinition>) {
    for switch in switches {
      if let Some(config) = &switch.config {
        let reporting = if config.inverted {
          SwitchReportingMode::ReportInverted
        } else {
          SwitchReportingMode::ReportNormal
        };
        log::info!("Configuring switch {} with {:?}", switch.name, config);
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

  async fn configure_drivers(io_port: &mut SerialInterface, drivers: &Vec<Driver>) {
    for driver in drivers {
      if let Some(config) = &driver.config {
        log::info!("Configuring driver {} with {:?}", driver.name, config);
        match io_port
          .request(
            &ConfigureDriverCommand::new(&driver.id, config),
            Duration::from_millis(500),
          )
          .await
        {
          Ok(ProcessedResponse::Processed) => {
            log::debug!("Driver {} configured successfully", driver.name);
          }
          Ok(ProcessedResponse::Failed) => {
            panic!("Driver {} configuration failed", driver.name);
          }
          Err(e) => {
            panic!("Error configuring driver {}: {}", driver.name, e);
          }
        }
      }
    }
  }

  async fn reset_expansion_boards(
    exp_port: &mut SerialInterface,
    expansion_boards: &Vec<ExpansionBoard>,
  ) {
    for board in expansion_boards {
      App::reset_expansion_board(exp_port, board).await;
    }
  }

  pub(crate) async fn reset_expansion_board(
    exp_port: &mut SerialInterface,
    board: &ExpansionBoard,
  ) {
    if board.breakout.is_none() {
      log::info!("Resetting expansion board at address {:X}", board.address);
      match exp_port
        .request(
          &BoardResetCommand::new(board.address),
          Duration::from_millis(2000),
        )
        .await
      {
        Ok(ProcessedResponse::Processed) => {
          log::debug!("Expansion board {:X} reset successfully", board.address);
        }
        Ok(ProcessedResponse::Failed) => {
          panic!(
            "Expansion board {:X} reset failed. Is this configured correctly?",
            board.address
          );
        }
        Err(e) => {
          panic!("Error resetting expansion board {:X}: {}", board.address, e);
        }
      }
    }
  }

  async fn configure_led_ports(
    exp_port: &mut SerialInterface,
    expansion_boards: &Vec<ExpansionBoard>,
  ) {
    for board in expansion_boards {
      for led_port in &board.led_ports {
        let cmd = ConfigureLedPortCommand::new(
          board.address,
          board.breakout,
          led_port.port,
          led_port.led_type.clone(),
          led_port.start,
          led_port.leds.len() as u8,
        );
        // configure port/block
        let _ = exp_port.request(&cmd, Duration::from_millis(250)).await;
      }
    }
  }

  pub fn operator_config(&mut self, item: impl OperatorConfigBuilder) -> &mut Self {
    let (key, config_item) = item.build();
    self.operator_config.add_item(key, config_item);
    self
  }

  pub fn system_tick(&mut self, interval: Duration) -> &mut Self {
    self.app_config.system_interval = interval;
    self
  }

  pub fn watchdog_tick(&mut self, interval: Duration) -> &mut Self {
    self.app_config.watchdog_interval = interval;
    self
  }

  pub fn system(&mut self, system: impl Into<SystemContainer>) -> &mut Self {
    self.systems.push(system.into());
    self
  }

  pub fn plugin(mut self, plugin: impl Plugin) -> Self {
    plugin.register(&mut self);
    self
  }

  pub fn configure(mut self, config_fn: impl FnOnce(&mut Self)) -> Self {
    config_fn(&mut self);
    self
  }

  pub async fn run(mut self) {
    log::debug!("Finalizing Store with operator config and app config");
    let context_base = ContextBase {
      switches: self.hardware.switches,
      drivers: self.hardware.drivers,
      io_network: self.hardware.io_network,
      exp_network: self.hardware.exp_network,
      app_config: self.app_config,
    };

    let (app_sender, app_receiver) = mpsc::unbounded_channel::<AppMessage>();
    let led_renderer = LedRenderer::new(&context_base.exp_network);

    let mut machine = MachineImpl::new(
      self.io_port,
      self.exp_port,
      context_base.clone(),
      app_sender.clone(),
      led_renderer,
    );
    let machine_sender = machine.machine_sender();

    // This needs to appear first to initialize all the commands that others systems expect to be present
    let bridge = Machine::new(machine_sender.clone());
    self.systems.insert(0, SystemContainer::new(bridge));

    // Start machine task
    tokio::spawn(async move {
      machine.run().await;
    });

    log::debug!("Starting main run loop");
    run_loop::run(
      context_base,
      self.systems,
      app_sender,
      app_receiver,
      machine_sender,
    )
    .await;
  }
}

#[derive(Debug, Clone, Serialize, Storable)]
pub struct AppConfig {
  /// The interval at which `on_tick` runs, which in turn affects timers and LED + display render speed
  pub system_interval: Duration,
  /// The interval at which the watchdog is pinged (keep alive)
  pub watchdog_interval: Duration,
}

impl Default for AppConfig {
  fn default() -> Self {
    Self {
      system_interval: Duration::from_millis(41),
      watchdog_interval: Duration::from_millis(1000),
    }
  }
}
