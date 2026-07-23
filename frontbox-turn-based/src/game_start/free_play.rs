use frontbox::prelude::*;
use frontbox::tags::StartButton;

use crate::GameManager;

/// A simple system to start a game in free play mode
#[derive(Clone)]
pub struct FreePlay {
  start_button_switch: HardwareQuery,
}

impl Default for FreePlay {
  fn default() -> Self {
    Self {
      start_button_switch: Q::tag::<StartButton>(),
    }
  }
}

impl FreePlay {
  pub fn new(selection: HardwareQuery) -> Self {
    Self {
      start_button_switch: selection,
    }
  }

  fn on_start_button_pressed(&mut self, ctx: &Context) {
    log::info!("Free play: Start button => add player");

    if let Some(mut game_management) = ctx.systems.get::<GameManager>() {
      if game_management.is_player_addable() {
        game_management.add_player(ctx);
      } else {
        log::debug!("GameManagement system reports player cannot be added, not adding player");
      }
    } else {
      log::warn!("Free play: GameManagement system not found, cannot start turn");
    }
  }
}

impl System for FreePlay {
  fn is_active(&self, ctx: &Context) -> bool {
    // active if players can be added
    ctx
      .systems
      .get::<GameManager>()
      .map(|gm| gm.is_player_addable())
      .unwrap_or(false)
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>()
      && self.start_button_switch.matches_switch(&e.switch)
    {
      self.on_start_button_pressed(ctx);
    }
  }
}
