use crate::modes::game_state::GameState;
use crate::store::Store;

// TODO: rename to PlayerContext?
#[derive(Debug)]
pub struct GameContext<'a> {
  game: &'a GameState,
  commands: Vec<GameCommand>,
  machine_store: &'a mut Store,
  player_store: &'a mut Store,
}

impl<'a> GameContext<'a> {
  pub fn new(
    game: &'a GameState,
    machine_store: &'a mut Store,
    player_store: &'a mut Store,
  ) -> Self {
    Self {
      game,
      commands: Vec::new(),
      machine_store,
      player_store,
    }
  }

  pub fn game(&self) -> &GameState {
    self.game
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

  pub fn configure_driver(&mut self) {
    self.commands.push(GameCommand::ConfigureDriver);
  }

  pub fn activate_driver(&mut self) {
    self.commands.push(GameCommand::ActivateDriver);
  }

  pub fn deactivate_driver(&mut self) {
    self.commands.push(GameCommand::DeactivateDriver);
  }

  pub fn trigger_driver(&mut self) {
    self.commands.push(GameCommand::TriggerDriver);
  }

  pub fn add_points(&mut self, points: u32) {
    self.commands.push(GameCommand::AddPoints(points));
  }

  // take_machine_commands
  // take_game_commands (take_player_commands?)
  pub(crate) fn take_commands(&mut self) -> Vec<GameCommand> {
    std::mem::take(&mut self.commands)
  }
}

// TODO: the driver commands belong on MachineCommand
#[derive(Debug, Clone)]
pub enum GameCommand {
  ConfigureDriver,
  ActivateDriver,
  DeactivateDriver,
  TriggerDriver,
  AddPoints(u32),
}
