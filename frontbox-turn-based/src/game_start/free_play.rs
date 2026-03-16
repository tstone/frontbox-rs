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
    if ctx.is(GameStartState::GameStartable) || ctx.is(GameStartState::PlayerAddable) {
      ctx.command(AddPlayer);
    }
  }
}

impl System for FreePlay {
  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {
    if let Some(e) = event.downcast::<SwitchClosed>() {
      if e.switch.name == self.start_button_id {
        self.on_start_button_pressed(ctx);
      }
    }
  }
}
