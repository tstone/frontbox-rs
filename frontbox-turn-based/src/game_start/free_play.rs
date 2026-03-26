use frontbox::prelude::*;

use crate::GameManager;

/// A simple system to start a game in free play mode
#[derive(Clone)]
pub struct FreePlay {
  start_button_id: &'static str,
}

impl FreePlay {
  pub fn new(start_button_id: &'static str) -> Self {
    Self { start_button_id }
  }

  fn on_start_button_pressed(&mut self, ctx: &Context, systems: &Systems) {
    log::info!("Free play: Start button => add player");

    if let Some(mut game_management) = systems.get_mut::<GameManager>() {
      if game_management.is_player_addable() {
        game_management.add_player(ctx, systems);
      } else {
        log::debug!("GameManagement system reports player cannot be added, not adding player");
      }
    } else {
      log::warn!("Free play: GameManagement system not found, cannot start turn");
    }
  }
}

impl System for FreePlay {
  fn is_active(&self, _ctx: &Context, systems: &Systems) -> bool {
    // active if players can be added
    systems
      .get::<GameManager>()
      .map(|gm| gm.is_player_addable())
      .unwrap_or(false)
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context, systems: &Systems) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      if e.switch.name == self.start_button_id {
        self.on_start_button_pressed(ctx, systems);
      }
    }
  }
}
