use std::collections::HashMap;

use tokio::sync::mpsc;

use crate::machine::*;
use crate::mainboard::*;
use crate::prelude::*;
use crate::store::Store;

pub struct MachineBuilder {
  command_tx: mpsc::Sender<MainboardCommand>,
  event_rx: mpsc::Receiver<MainboardIncoming>,
  switches: SwitchContext,
  driver_lookup: HashMap<&'static str, DriverPin>,
  keyboard_switch_map: HashMap<KeyCode, usize>,
  root_machine_scene: Scene,
  root_game_scene: Scene,
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
      root_machine_scene: Vec::new(),
      root_game_scene: Vec::new(),
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

  pub fn add_root_system(
    &mut self,
    system: impl System + 'static,
    scope: SystemScope,
  ) -> &mut Self {
    match scope {
      SystemScope::Machine => self.root_machine_scene.push(Box::new(system)),
      SystemScope::Game => self.root_game_scene.push(Box::new(system)),
    }
    self
  }

  pub fn build(self) -> Machine {
    Machine {
      command_tx: self.command_tx.clone(),
      event_rx: self.event_rx,
      switches: self.switches,
      driver_lookup: self.driver_lookup,
      keyboard_switch_map: self.keyboard_switch_map,
      machine_stack: vec![self.root_machine_scene],
      init_game_stack: vec![self.root_game_scene],
      current_game_stack: Vec::new(),
      store: Store::new(),
      game_state: None,
      mode: MachineMode::Attract,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SystemScope {
  /// A system that runs at the scope of the machine, active regardless of game state
  Machine,
  /// A system that runs within a game, unique to a player or team
  Game,
}
