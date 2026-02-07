use crate::prelude::*;

/// A runtime manages which stack of scenes is currently active, acting as a switchboard operator
#[allow(unused)]
pub trait Runtime {
  // fn current_scene(&mut self) -> &mut Scene;
  // fn current_store(&mut self) -> &mut Store;
  fn get_current(&mut self) -> (&mut Scene, &mut Store);

  fn on_runtime_enter(&self, ctx: &mut RuntimeContext) {}
  fn on_add_player(&mut self, player_index: u8) {}
  fn on_change_player(&mut self, player_index: u8) {}
  fn on_runtime_exit(&mut self, ctx: &mut RuntimeContext) {}
}

pub struct RuntimeContext {
  commands: Vec<RuntimeCommand>,
}

impl RuntimeContext {
  pub fn new() -> Self {
    Self {
      commands: Vec::new(),
    }
  }

  pub fn start_game(&mut self) {
    self.commands.push(RuntimeCommand::StartGame);
  }

  pub(crate) fn commands(&mut self) -> Vec<RuntimeCommand> {
    self.commands.drain(..).collect()
  }
}

pub enum RuntimeCommand {
  StartGame,
}
