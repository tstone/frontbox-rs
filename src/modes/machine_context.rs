use crate::store::Store;
use crate::modes::game_state::GameState;

#[derive(Debug)]
pub struct MachineContext<'a> {
  game: &'a GameState,
  commands: Vec<MachineCommand>,
  store: &'a mut Store,
}

impl<'a> MachineContext<'a> {
  pub fn new(game: &'a GameState, store: &'a mut Store) -> Self {
    Self {
      game,
      commands: Vec::new(),
      store,
    }
  }

  pub fn game(&self) -> &GameState {
    self.game
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

  pub(crate) fn take_commands(&mut self) -> Vec<MachineCommand> {
    std::mem::take(&mut self.commands)
  }
}

#[derive(Debug, Clone)]
pub enum MachineCommand {
  StartGame,
  AddPlayer,
}
