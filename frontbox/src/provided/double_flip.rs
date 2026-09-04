use crate::prelude::*;

/// A system that emits a DoubleFlip event when the left and right flippers are pressed at about the same time.
pub struct DoubleFlipSystem {
  l_flip_q: SwitchQ,
  r_flip_q: SwitchQ,
  l_flip: bool,
  r_flip: bool,
  cue_id: Option<u64>,
}

impl DoubleFlipSystem {
  pub fn new(l_flip_q: SwitchQ, r_flip_q: SwitchQ) -> Self {
    Self {
      l_flip_q,
      r_flip_q,
      l_flip: false,
      r_flip: false,
      cue_id: None,
    }
  }

  fn cancel_cue(&mut self, ctx: &SystemContext) {
    if let Some(cue_id) = self.cue_id {
      ctx.cancel_cue(cue_id);
      self.cue_id = None;
    }
  }

  fn cue(&mut self, ctx: &SystemContext) {
    self.cue_id = Some(ctx.cue(TimeOut, Duration::from_millis(250).once()));
  }

  fn left_flip(&mut self, ctx: &SystemContext) {
    if self.r_flip {
      self.emit(ctx);
    } else if !self.l_flip {
      self.cancel_cue(ctx);
      self.l_flip = true;
      self.cue(ctx);
    }
  }

  fn right_flip(&mut self, ctx: &SystemContext) {
    if self.l_flip {
      self.emit(ctx);
    } else if !self.r_flip {
      self.cancel_cue(ctx);
      self.r_flip = true;
      self.cue(ctx);
    }
  }

  fn emit(&mut self, ctx: &SystemContext) {
    ctx.emit(DoubleFlip);
    self.cancel_cue(ctx);
  }

  fn reset(&mut self, ctx: &SystemContext) {
    self.l_flip = false;
    self.r_flip = false;
    self.cancel_cue(ctx);
  }
}

impl System for DoubleFlipSystem {
  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(event) = event.downcast_ref::<SwitchClosed>() {
      if self.l_flip_q.matches(&event.switch) {
        self.left_flip(ctx);
      } else if self.r_flip_q.matches(&event.switch) {
        self.right_flip(ctx);
      }
    } else if event.is::<TimeOut>() {
      self.reset(ctx);
    }
  }
}

#[derive(serde::Serialize, Event)]
struct TimeOut;

#[derive(serde::Serialize, Event)]
struct DoubleFlip;
