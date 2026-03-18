use frontbox::prelude::*;

use crate::{AddPlayer, GameStartState};

/// A simple system to start a game in free play mode
#[derive(Clone)]
pub struct FreePlay {
  start_button_id: &'static str,
}

impl FreePlay {
  pub fn new(start_button_id: &'static str) -> Box<Self> {
    Box::new(Self { start_button_id })
  }

  fn on_start_button_pressed(&mut self, ctx: &mut Context) {
    log::info!("Free play: Start button => add player");
    ctx.command(AddPlayer);
  }
}

impl System for FreePlay {
  fn is_active(&self, ctx: &Context) -> bool {
    let has_start_state = ctx.has::<GameStartState>();

    if !has_start_state {
      log::warn!("FreePlay expects GameStartState, but is missing from context");
      return false;
    }

    ctx.is(GameStartState::GameStartable) || ctx.is(GameStartState::PlayerAddable)
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      if e.switch.name == self.start_button_id {
        self.on_start_button_pressed(ctx);
      }
    }
  }
}
