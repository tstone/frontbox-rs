use frontbox_core::prelude::*;

use crate::{GameManager, PlayerEntry};

/// A simple system to start a game in free play mode
#[derive(Clone)]
pub struct FreePlay {
  start_button_switch: SwitchQ,
}

impl FreePlay {
  pub fn new(q: SwitchQ) -> Self {
    Self {
      start_button_switch: q,
    }
  }

  fn on_start_button_pressed(&mut self, ctx: &SystemContext) {
    log::info!(target: "frontbox::game_manager", "Free play: Start button => add player");

    // only active while `PlayerEntry` is open, and the game manager ignores a press that comes in as it closes
    if let Some(mut game_management) = ctx.systems.get::<GameManager>() {
      game_management.add_player(ctx.into());
    } else {
      log::warn!(target: "frontbox::game_manager", "Free play: GameManagement system not found, cannot start turn");
    }
  }
}

impl System for FreePlay {
  fn is_active(&self, ctx: &SystemContext) -> bool {
    ctx.states.is(PlayerEntry::Open)
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>()
      && self.start_button_switch.matches(&e.switch)
    {
      self.on_start_button_pressed(ctx);
    }
  }
}
