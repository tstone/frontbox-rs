use dyn_clone::DynClone;

use crate::prelude::*;

/// A System responds to incoming events and enqueues commands
#[allow(unused)]
pub trait System: DynClone + Send + Sync {
  /// Used by the machine to determine if this mode should receive events
  fn is_listening(&self) -> bool {
    true
  }

  /// Called when a switch becomes closed (depressed). Affected by is_listening
  fn event_switch_closed(
    &mut self,
    switch: &Switch,
    ctx: &mut Context,
    game: Option<&mut GameState>,
  ) {
  }
  /// Called when a switch becomes open (released). Affected by is_listening
  fn event_switch_opened(
    &mut self,
    switch: &Switch,
    ctx: &mut Context,
    game: Option<&mut GameState>,
  ) {
  }

  fn on_game_start(&mut self, ctx: &mut Context, game: &mut GameState) {}
  fn on_game_end(&mut self, ctx: &mut Context, game: &mut GameState) {}
  fn on_ball_start(&mut self, ctx: &mut Context, game: &mut GameState) {}
  fn on_ball_end(&mut self, ctx: &mut Context, game: &mut GameState) {}
}
