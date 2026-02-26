use std::collections::HashMap;
use std::time::Duration;

use crate::hardware_definition::*;
use crate::machine::serial_interface::SerialInterface;
use crate::machine::switch_context::SwitchContext;
use crate::prelude::*;
use crate::protocol::SwitchState;
use crate::protocol::prelude::*;

pub struct MachineBuilder {
  io_port: SerialInterface,
  exp_port: SerialInterface,
  switches: SwitchContext,
  driver_lookup: HashMap<&'static str, Driver>,
  keyboard_switch_map: HashMap<KeyCode, usize>,
  virtual_switch_count: u8,
  config: MachineConfig,
  expansion_boards: Vec<ExpansionBoardSpec>,
  districts: HashMap<&'static str, Box<dyn District>>,
}

impl MachineBuilder {
  pub async fn boot(
    config: BootConfig,
    io_network: IoNetwork,
    expansion_boards: Vec<ExpansionBoardSpec>,
  ) -> Self {
    let mut io_port = SerialInterface::new(config.io_net_port_path)
      .await
      .expect("Failed to open IO NET port");
    log::info!("🥾 Opened IO NET port at {}", config.io_net_port_path);

    MachineBuilder::boot_mainboard(&mut io_port).await;
    MachineBuilder::configure_hardware(&mut io_port, config.platform).await;
    MachineBuilder::verify_watchdog(&mut io_port).await;
    MachineBuilder::configure_switches(&mut io_port, &io_network.switches).await;

    // Initialize switch context which Machine will use to maintain current state
    let initial_switch_state = MachineBuilder::get_initial_switch_states(&mut io_port).await;
    let switches = SwitchContext::new(io_network.switches, initial_switch_state);

    // Configure drivers
    MachineBuilder::configure_drivers(&mut io_port, &io_network.drivers).await;
    let mut drivers = HashMap::new();
    for driver in io_network.drivers {
      drivers.insert(driver.name, driver);
    }

    // open EXP port
    let mut exp_port = SerialInterface::new(config.exp_port_path)
      .await
      .expect("Failed to open EXP port");
    log::info!("🥾 Opened EXP port at {}", config.exp_port_path);

    MachineBuilder::reset_expansion_boards(&mut exp_port, &expansion_boards).await;
    MachineBuilder::configure_led_ports(&mut exp_port, &expansion_boards).await;

    Self {
      io_port,
      exp_port,
      switches,
      driver_lookup: drivers,
      keyboard_switch_map: HashMap::new(),
      virtual_switch_count: 0,
      config: MachineConfig::default(),
      expansion_boards,
      districts: HashMap::new(),
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

  async fn configure_switches(io_port: &mut SerialInterface, switches: &Vec<SwitchSpec>) {
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

  pub async fn reset_expansion_boards(
    exp_port: &mut SerialInterface,
    expansion_boards: &Vec<ExpansionBoardSpec>,
  ) {
    for board in expansion_boards {
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
  }

  async fn configure_led_ports(
    exp_port: &mut SerialInterface,
    expansion_boards: &Vec<ExpansionBoardSpec>,
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

  /// Map a keyboard key to a switch for emulated switch triggering
  pub fn add_keyboard_mapping(mut self, key: KeyCode, switch_name: &'static str) -> Self {
    let switch = self.switches.switch_by_name(switch_name).expect(&format!(
      "Keyboard mapped switch '{}' not found.",
      switch_name
    ));
    self.keyboard_switch_map.insert(key, switch.id);
    self
  }

  pub fn add_keyboard_mappings(mut self, mappings: Vec<(KeyCode, &'static str)>) -> Self {
    for (key, switch_name) in mappings {
      self = self.add_keyboard_mapping(key, switch_name);
    }
    self
  }

  /// Add a virtual switch that can be triggered by a keyboard key which is not backed by a hardware switch.
  /// Used primarily for testing or to emulate future hardware before it's physically installed.
  pub fn add_virtual_switch(mut self, key: KeyCode, switch_name: &'static str) -> Self {
    if self.virtual_switch_count == u8::MAX {
      panic!("Maximum number of virtual switches added");
    }

    // Virtual IDs count backwards from the max ID size to avoid colliding with hardware switch IDs which start at 0 and increment upwards
    let virtual_id = usize::MAX - self.virtual_switch_count as usize;
    self.switches.add_virtual_switch(switch_name, virtual_id);
    self.keyboard_switch_map.insert(key, virtual_id);

    self.virtual_switch_count += 1;
    self
  }

  /// Add a virtual switches that can be triggered by a keyboard key which is not backed by a hardware switch.
  /// Used primarily for testing or to emulate future hardware before it's physically installed.
  pub fn add_virtual_switches(mut self, mappings: Vec<(KeyCode, &'static str)>) -> Self {
    for (key, switch_name) in mappings {
      self = self.add_virtual_switch(key, switch_name);
    }
    self
  }

  pub fn add_config_item(mut self, key: &'static str, item: ConfigItem) -> Self {
    self.config.add_item(key, item);
    self
  }

  pub fn set_config_value(mut self, key: &'static str, value: ConfigValue) -> Self {
    self.config.set_value(key, value);
    self
  }

  pub fn add_plugin(mut self, plugin: Box<dyn Plugin>) -> Self {
    plugin.register(&mut self);
    self
  }

  pub fn insert_district(mut self, key: &'static str, district: Box<dyn District>) -> Self {
    self.districts.insert(key, district);
    self
  }

  pub fn build(self) -> Machine {
    Machine::new(
      self.io_port,
      self.exp_port,
      self.switches,
      self.driver_lookup,
      self.keyboard_switch_map,
      self.config,
      self.expansion_boards,
      self.districts,
    )
  }
}
