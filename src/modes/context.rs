use crate::hardware::driver_config::DriverConfig;
use crate::machine::*;
use crate::prelude::*;
use crate::store::Store;

pub struct Context<'a> {
  mode: &'a MachineMode,
  commands: Vec<Box<dyn Command + 'static>>,
  store: &'a mut Store,
  switches: &'a SwitchContext,
}

impl<'a> Context<'a> {
  pub fn new(mode: &'a MachineMode, store: &'a mut Store, switches: &'a SwitchContext) -> Self {
    Self {
      mode,
      commands: Vec::new(),
      store,
      switches,
    }
  }

  pub fn is_game_started(&self) -> bool {
    self.mode == MachineMode::Game
  }

  pub fn in_attract_mode(&self) -> bool {
    self.mode == MachineMode::Attract
  }

  pub fn in_admin_mode(&self) -> bool {
    self.mode == MachineMode::Admin
  }

  pub fn mode(&self) -> &MachineMode {
    &self.mode
  }

  pub fn is_switch_closed(&self, switch_name: &'static str) -> Option<bool> {
    self.switches.is_closed_by_name(switch_name)
  }

  pub fn is_switch_open(&self, switch_name: &'static str) -> Option<bool> {
    self.switches.is_open_by_name(switch_name)
  }

  pub fn command(&mut self, command: impl Command + 'static) {
    self.commands.push(Box::new(command));
  }

  // TODO: broadcast event bus
  // pub fn emit() {

  // }

  pub fn get<T: Default + 'static>(&mut self) -> &T {
    self.store.get_state::<T>()
  }

  pub fn get_mut<T: Default + 'static>(&mut self) -> &mut T {
    self.store.get_state_mut::<T>()
  }

  pub fn insert<T: Default + 'static>(&mut self, value: T) {
    self.store.insert_state::<T>(value);
  }

  pub fn remove<T: Default + 'static>(&mut self) {
    self.store.remove_state::<T>();
  }

  pub(crate) fn take_commands(&mut self) -> Vec<Box<dyn Command>> {
    std::mem::take(&mut self.commands)
  }

  // pub fn start_game(&mut self) {
  //   self.commands_old.push(MachineCommand::StartGame);
  // }

  // pub fn add_player(&mut self) {
  //   self.commands_old.push(MachineCommand::AddPlayer);
  // }

  // pub fn activate_high_voltage(&mut self) {
  //   self.commands_old.push(MachineCommand::ActivateHighVoltage);
  // }

  // pub fn deactivate_high_voltage(&mut self) {
  //   self
  //     .commands_old
  //     .push(MachineCommand::DeactivateHighVoltage);
  // }

  // pub fn configure_driver(&mut self, driver: &'static str, config: DriverConfig) {
  //   self
  //     .commands_old
  //     .push(MachineCommand::ConfigureDriver(driver, config));
  // }

  // pub fn activate_driver(&mut self, driver: &'static str) {
  //   self
  //     .commands_old
  //     .push(MachineCommand::ActivateDriver(driver));
  // }

  // pub fn deactivate_driver(&mut self, driver: &'static str) {
  //   self
  //     .commands_old
  //     .push(MachineCommand::DeactivateDriver(driver));
  // }

  // pub fn trigger_driver(&mut self, driver: &'static str) {
  //   self
  //     .commands_old
  //     .push(MachineCommand::TriggerDriver(driver));
  // }

  // pub fn add_points(&mut self, points: u32) {
  //   self.commands_old.push(MachineCommand::AddPoints(points));
  // }

  // pub fn next_player(&mut self) {
  //   self.commands_old.push(MachineCommand::NextPlayer);
  // }
}

#[derive(Debug, Clone)]
pub enum MachineCommand {
  StartGame,
  AddPlayer,
  ActivateHighVoltage,
  DeactivateHighVoltage,
  ConfigureDriver(&'static str, DriverConfig),
  ActivateDriver(&'static str),
  DeactivateDriver(&'static str),
  TriggerDriver(&'static str),

  // In-game commands
  AddPoints(u32),
  NextPlayer,
  // ExtraBall,
}
