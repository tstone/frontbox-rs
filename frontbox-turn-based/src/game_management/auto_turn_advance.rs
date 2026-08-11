use frontbox::prelude::*;

use crate::{GameManager, PlayerTurnEnding};

/// Typically you'd want to have some sort of "end of ball" display or sequence, after which time you'd call `advance_turn` on the `GameManager`.
/// This system is just a simple example of how you could automatically advance the turn after a `PlayerTurnEnding` event is emitted. Use this
/// when you're first setting up a game and just want to get something working, but you'll likely want to replace this with something more complex in a real game.
#[derive(Default)]
pub struct AutoTurnAdvance;

impl AutoTurnAdvance {
  pub fn new() -> Self {
    Self
  }
}

impl System for AutoTurnAdvance {
  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<PlayerTurnEnding>()
      && let Some(mut game_manager) = ctx.get::<GameManager>()
    {
      game_manager.advance_turn(ctx);
    }
  }
}
