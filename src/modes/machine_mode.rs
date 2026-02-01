use std::fmt::Debug;

use crate::machine::Switch;
use crate::modes::game_state::GameState;
use crate::modes::machine_context::MachineContext;

#[allow(unused)]
pub trait MachineMode: Debug {
  fn is_active(&self) -> bool {
    true
  }

  fn on_switch_activated(&mut self, switch: &Switch, ctx: &mut MachineContext) {}
  fn on_switch_deactivated(&mut self, switch: &Switch, ctx: &mut MachineContext) {}
  fn on_game_state_changed(&mut self, ctx: &mut GameState) {}
}
