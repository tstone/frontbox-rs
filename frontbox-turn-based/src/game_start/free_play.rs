use frontbox::prelude::*;

use crate::GameManager;

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
    log::info!("Free play: Start button => add player");

    if let Some(mut game_management) = ctx.get::<GameManager>() {
      if game_management.is_player_addable() {
        game_management.add_player(ctx.into());
      } else {
        log::debug!("GameManagement system reports player cannot be added, not adding player");
      }
    } else {
      log::warn!("Free play: GameManagement system not found, cannot start turn");
    }
  }
}

impl System for FreePlay {
  fn is_active(&self, ctx: &SystemContext) -> bool {
    // active if players can be added
    ctx
      .get::<GameManager>()
      .map(|gm| gm.is_player_addable())
      .unwrap_or(false)
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>()
      && self.start_button_switch.matches(&e.switch)
    {
      self.on_start_button_pressed(ctx);
    }
  }
}
