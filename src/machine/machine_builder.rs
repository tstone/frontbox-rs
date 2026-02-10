use std::collections::HashMap;

use tokio::sync::mpsc;

use crate::machine::switch_context::SwitchContext;
use crate::mainboard::*;
use crate::prelude::*;
use crate::protocol;

pub struct MachineBuilder {
  command_tx: mpsc::Sender<MainboardCommand>,
  event_rx: mpsc::Receiver<MainboardIncoming>,
  switches: SwitchContext,
  driver_lookup: HashMap<&'static str, Driver>,
  keyboard_switch_map: HashMap<KeyCode, usize>,
  virtual_switch_count: u8,
}

impl MachineBuilder {
  pub async fn boot(config: BootConfig, io_network: IoNetwork) -> Self {
    let (command_tx, command_rx) = mpsc::channel::<MainboardCommand>(128);
    let (event_tx, event_rx) = mpsc::channel::<MainboardIncoming>(128);

    let BootResult {
      mut mainboard,
      initial_switch_state,
    } = Mainboard::boot(config, command_rx, event_tx.clone()).await;

    // start serial communication in separate thread
    std::thread::spawn(move || {
      let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

      runtime.block_on(async move {
        mainboard.run().await;
      });
    });

    // Configure switch reporting/bounce settings
    for switch in &io_network.switches {
      if let Some(config) = &switch.config {
        let reporting = if config.inverted {
          protocol::configure_switch::SwitchReportingMode::ReportInverted
        } else {
          protocol::configure_switch::SwitchReportingMode::ReportNormal
        };
        log::info!("Configuring switch {} with {:?}", switch.name, config);
        let cmd = protocol::configure_switch::request(
          switch.id,
          reporting,
          config.debounce_close,
          config.debounce_open,
        );
        command_tx
          .send(MainboardCommand::SendIo(cmd))
          .await
          .unwrap();
      }
    }

    // Initialize switch context which Machine will use to maintain current state
    let switches = SwitchContext::new(io_network.switches, initial_switch_state);

    // Configure drivers
    for driver in &io_network.drivers {
      if let Some(config) = &driver.config {
        log::info!("Configuring driver {} with {:?}", driver.name, config);
        let cmd = protocol::configure_driver::request(&driver.id, config);
        command_tx
          .send(MainboardCommand::SendIo(cmd))
          .await
          .unwrap();
      }
    }

    // TODO: define LEDs

    let mut drivers = HashMap::new();
    for driver in io_network.drivers {
      drivers.insert(driver.name, driver);
    }

    Self {
      command_tx,
      event_rx,
      switches,
      driver_lookup: drivers,
      keyboard_switch_map: HashMap::new(),
      virtual_switch_count: 0,
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

  pub fn add_plugin(mut self, plugin: Box<dyn Plugin>) -> Self {
    plugin.register(&mut self);
    self
  }

  pub fn build(self) -> Machine {
    Machine {
      command_tx: self.command_tx.clone(),
      event_rx: self.event_rx,
      switches: self.switches,
      driver_lookup: self.driver_lookup,
      keyboard_switch_map: self.keyboard_switch_map,
      runtime_stack: Vec::new(),
      active_player: -1,
      active_player_count: 0,
    }
  }
}
