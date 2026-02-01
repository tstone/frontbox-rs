use crate::hardware::driver_config::DriverConfig;
use crate::modes::game_state::GameState;
use crate::store::Store;
use crate::switch_context::SwitchContext;

#[derive(Debug)]
pub struct MachineContext<'a> {
  game: &'a GameState,
  commands: Vec<MachineCommand>,
  store: &'a mut Store,
  switches: &'a SwitchContext,
}

impl<'a> MachineContext<'a> {
  pub fn new(game: &'a GameState, store: &'a mut Store, switches: &'a SwitchContext) -> Self {
    Self {
      game,
      commands: Vec::new(),
      store,
      switches,
    }
  }

  pub fn game(&self) -> &GameState {
    self.game
  }

  pub fn is_switch_closed(&self, switch_name: &'static str) -> Option<bool> {
    self.switches.is_closed_by_name(switch_name)
  }

  pub fn is_switch_open(&self, switch_name: &'static str) -> Option<bool> {
    self.switches.is_open_by_name(switch_name)
  }

  pub fn store_get<T: Default + 'static>(&mut self) -> &T {
    self.store.get_state::<T>()
  }

  pub fn store_get_mut<T: Default + 'static>(&mut self) -> &mut T {
    self.store.get_state_mut::<T>()
  }

  pub fn store_insert<T: Default + 'static>(&mut self, value: T) {
    self.store.insert_state::<T>(value);
  }

  pub fn start_game(&mut self) {
    self.commands.push(MachineCommand::StartGame);
  }

  pub fn add_player(&mut self) {
    self.commands.push(MachineCommand::AddPlayer);
  }

  pub fn activate_high_voltage(&mut self) {
    self.commands.push(MachineCommand::ActivateHighVoltage);
  }

  pub fn deactivate_high_voltage(&mut self) {
    self.commands.push(MachineCommand::DeactivateHighVoltage);
  }

  pub fn configure_driver(&mut self, driver: &'static str, config: DriverConfig) {
    self
      .commands
      .push(MachineCommand::ConfigureDriver(driver, config));
  }

  pub fn activate_driver(&mut self, driver: &'static str) {
    self.commands.push(MachineCommand::ActivateDriver(driver));
  }

  pub fn deactivate_driver(&mut self, driver: &'static str) {
    self.commands.push(MachineCommand::DeactivateDriver(driver));
  }

  pub fn trigger_driver(&mut self, driver: &'static str) {
    self.commands.push(MachineCommand::TriggerDriver(driver));
  }

  pub(crate) fn take_commands(&mut self) -> Vec<MachineCommand> {
    std::mem::take(&mut self.commands)
  }
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
}
