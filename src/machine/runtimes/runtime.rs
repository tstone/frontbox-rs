use crate::prelude::*;

/// A runtime manages which stack of scenes is currently active, acting as a switchboard operator
#[allow(unused)]
pub trait Runtime {
  fn current_scene(&mut self) -> &mut Scene;

  fn on_add_player(&mut self, player_index: u8) {}
  fn on_change_player(&mut self, player_index: u8) {}
  fn on_runtime_exit(&mut self, machine: &mut Machine) {}
}
