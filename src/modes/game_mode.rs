use dyn_clone::DynClone;
use std::fmt::Debug;

use crate::machine::Switch;
use crate::modes::game_state::GameState;
use crate::prelude::MachineContext;
use crate::switch_context::SwitchContext;

#[allow(unused)]
pub trait GameMode: Debug + DynClone {
  /// Used by the machine to determine if this mode should receive events
  fn is_listening(&self) -> bool {
    true
  }

  /// Called when a switch becomes closed (depressed). Affected by is_listening, is_active.
  fn event_switch_closed(&mut self, switch: &Switch, ctx: &mut MachineContext) {}
  /// Called when a switch becomes open (released). Affected by is_listening, is_active.
  fn event_switch_opened(&mut self, switch: &Switch, ctx: &mut MachineContext) {}

  /// Called when the game state changes. Affected by is_active.
  fn on_game_state_changed(&mut self, old: &GameState, new: &GameState, ctx: &SwitchContext) {}
}
