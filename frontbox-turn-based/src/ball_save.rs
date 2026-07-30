use frontbox::prelude::*;
use frontbox::provided::TroughFull;

use crate::{PlayerTurnActive, PlayerTurnEnding};

/// Standard ball save at the start of a ball
#[derive(Clone)]
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

  pub fn new(initial_duration: Duration) -> Self {
    Self {
      duration: initial_duration,
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
      effect.resume();
    }

    ctx.register_interrupt::<TroughFull>(Self::trough_interrupt_priority());
    ctx.cue(EndBallSave, Cue::Once(self.duration));
  }

  pub fn deactivate(&mut self, ctx: &Context) {
    self.active = false;
    ctx.unregister_interrupt::<TroughFull>();

    if let Some(cue_id) = self.cue {
      ctx.cancel_cue(cue_id);
    }

    for effect in &mut self.effects {
      effect.stop_and_clear(ctx);
    }
  }

  pub fn duration_mut(&mut self) -> &mut Duration {
    &mut self.duration
  }
}

impl System for BallSaveSystem {
  fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
    if event.downcast_ref::<PlayerTurnActive>().is_some() {
      // automatically do ball save at the start of a turn
      self.activate(ctx);
    } else if event.is::<PlayerTurnEnding>() || event.is::<EndBallSave>() {
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
