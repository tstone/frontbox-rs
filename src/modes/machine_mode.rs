use std::fmt::Debug;

use crate::machine::Switch;
use crate::modes::game_state::GameState;
use crate::modes::machine_context::MachineContext;
use crate::switch_context::SwitchContext;

#[allow(unused)]
pub trait MachineMode: Debug {
  /// Used by the machine to determine if this mode should receive events
  fn is_listening(&self) -> bool {
    true
  }

  /// Called when a switch becomes closed (depressed). Affected by is_listening
  fn event_switch_closed(&mut self, switch: &Switch, ctx: &mut MachineContext) {}
  /// Called when a switch becomes open (released). Affected by is_listening
  fn event_switch_opened(&mut self, switch: &Switch, ctx: &mut MachineContext) {}

  /// Called when the game state changes. Not affected by is_listening
  fn on_game_state_changed(&mut self, old: &GameState, new: &GameState, switches: &SwitchContext) {}
}
