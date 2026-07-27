use frontbox::prelude::*;
use frontbox::provided::TroughFull;

use crate::{PlayerTurnActive, PlayerTurnEnding};

/// Standard ball save at the start of a ball
pub struct BallSaveSystem {
  duration: Duration,
  effects: Vec<LedEffect>,
  cue: Option<u64>,
  active: bool,
}

impl BallSaveSystem {
  pub fn trough_interrupt_priority() -> u16 {
    25
  }

  pub fn new(duration: Duration) -> Self {
    Self {
      duration,
      effects: Vec::new(),
      cue: None,
      active: false,
    }
  }

  pub fn effect(mut self, effect: LedEffect) -> Self {
    self.effects.push(effect);
    self
  }

  pub fn activate(&mut self, ctx: &Context) {
    self.active = true;

    for effect in &mut self.effects {
      effect.play();
    }

    ctx.register_interrupt::<TroughFull>(Self::trough_interrupt_priority());
    ctx.cue(EndBallSave, Cue::Once(self.duration.clone()));
  }

  pub fn deactivate(&mut self, ctx: &Context) {
    self.active = false;

    if let Some(cue_id) = self.cue {
      ctx.cancel_cue(cue_id);
    }

    ctx.unregister_interrupt::<TroughFull>();
  }
}

impl System for BallSaveSystem {
  fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
    if event.downcast_ref::<PlayerTurnActive>().is_some() {
      // automatically do ball save at the start of a turn
      self.activate(ctx);
    } else if event.is::<PlayerTurnEnding>() {
      // cancel any pending ball saves if the turn is over
      self.deactivate(ctx);
    }
  }

  fn on_interrupt(&mut self, _event: &dyn Event, _ctx: &Context) -> InterruptResult {
    // while active all TroughFull events are stopped
    InterruptResult::Halt
  }

  fn on_tick(&mut self, delta: Duration, ctx: &Context) {
    if self.active {
      for effect in &mut self.effects {
        effect.apply(delta, ctx);
      }
    }
  }
}

struct EndBallSave;
