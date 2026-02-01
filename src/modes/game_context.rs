use crate::hardware::driver_config::DriverConfig;
use crate::modes::game_state::GameState;
use crate::modes::machine_context::MachineCommand;
use crate::store::Store;
use crate::switch_context::SwitchContext;

#[derive(Debug)]
pub struct GameContext<'a> {
  game: &'a GameState,
  machine_commands: Vec<MachineCommand>,
  game_commands: Vec<GameCommand>,
  machine_store: &'a mut Store,
  player_store: &'a mut Store,
  switches: &'a SwitchContext,
}

impl<'a> GameContext<'a> {
  pub fn new(
    game: &'a GameState,
    machine_store: &'a mut Store,
    player_store: &'a mut Store,
    switches: &'a SwitchContext,
  ) -> Self {
    Self {
      game,
      machine_commands: Vec::new(),
      game_commands: Vec::new(),
      machine_store,
      player_store,
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

  pub fn machine_store_get<T: Default + 'static>(&mut self) -> &T {
    self.machine_store.get_state::<T>()
  }

  pub fn machine_store_get_mut<T: Default + 'static>(&mut self) -> &mut T {
    self.machine_store.get_state_mut::<T>()
  }

  pub fn machine_store_insert<T: Default + 'static>(&mut self, value: T) {
    self.machine_store.insert_state::<T>(value);
  }

  pub fn player_store_get<T: Default + 'static>(&mut self) -> &T {
    self.player_store.get_state::<T>()
  }

  pub fn player_store_get_mut<T: Default + 'static>(&mut self) -> &mut T {
    self.player_store.get_state_mut::<T>()
  }

  pub fn player_store_insert<T: Default + 'static>(&mut self, value: T) {
    self.player_store.insert_state::<T>(value);
  }

  pub fn configure_driver(&mut self, driver: &'static str, config: DriverConfig) {
    self
      .machine_commands
      .push(MachineCommand::ConfigureDriver(driver, config));
  }

  pub fn activate_driver(&mut self, driver: &'static str) {
    self
      .machine_commands
      .push(MachineCommand::ActivateDriver(driver));
  }

  pub fn deactivate_driver(&mut self, driver: &'static str) {
    self
      .machine_commands
      .push(MachineCommand::DeactivateDriver(driver));
  }

  pub fn trigger_driver(&mut self, driver: &'static str) {
    self
      .machine_commands
      .push(MachineCommand::TriggerDriver(driver));
  }

  pub fn add_points(&mut self, points: u32) {
    self.game_commands.push(GameCommand::AddPoints(points));
  }

  pub(crate) fn take_machine_commands(&mut self) -> Vec<MachineCommand> {
    std::mem::take(&mut self.machine_commands)
  }

  pub(crate) fn take_game_commands(&mut self) -> Vec<GameCommand> {
    std::mem::take(&mut self.game_commands)
  }
}

#[derive(Debug, Clone)]
pub enum GameCommand {
  AddPoints(u32),
  IncrementPlayer,
}
