use std::collections::HashMap;

use tokio::sync::mpsc;

use crate::machine::switch_context::SwitchContext;
use crate::mainboard::*;
use crate::prelude::*;

pub struct MachineBuilder {
  command_tx: mpsc::Sender<MainboardCommand>,
  event_rx: mpsc::Receiver<MainboardIncoming>,
  switches: SwitchContext,
  driver_lookup: HashMap<&'static str, DriverPin>,
  keyboard_switch_map: HashMap<KeyCode, usize>,
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

    // TODO: define LEDs
    let switches = SwitchContext::new(io_network.switches, initial_switch_state);

    let mut drivers = HashMap::new();
    for driver in io_network.driver_pins {
      drivers.insert(driver.name, driver);
    }

    Self {
      command_tx,
      event_rx,
      switches,
      driver_lookup: drivers,
      keyboard_switch_map: HashMap::new(),
    }
  }

  pub fn add_keyboard_mapping(&mut self, key: KeyCode, switch_name: &'static str) -> &mut Self {
    let switch = self.switches.switch_by_name(switch_name).unwrap();
    self.keyboard_switch_map.insert(key, switch.id);
    self
  }

  pub fn add_keyboard_mappings(&mut self, mappings: Vec<(KeyCode, &'static str)>) -> &mut Self {
    for (key, switch_name) in mappings {
      self.add_keyboard_mapping(key, switch_name);
    }
    self
  }

  pub fn add_plugin(&mut self, plugin: Box<dyn Plugin>) -> &mut Self {
    plugin.register(self);
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
