use dyn_clone::DynClone;

use crate::prelude::*;

/// A System responds to incoming events and enqueues commands
#[allow(unused)]
pub trait System: DynClone + Send + Sync {
  /// Called when a switch becomes closed (depressed)
  fn on_switch_closed(&mut self, switch: &Switch, ctx: &mut Context) {}
  /// Called when a switch becomes open (released)
  fn on_switch_opened(&mut self, switch: &Switch, ctx: &mut Context) {}

  fn on_game_start(&mut self, ctx: &mut Context) {}
  fn on_game_end(&mut self, ctx: &mut Context) {}
  fn on_ball_start(&mut self, ctx: &mut Context) {}
  fn on_ball_end(&mut self, ctx: &mut Context) {}
}
