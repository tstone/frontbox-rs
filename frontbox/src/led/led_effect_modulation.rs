//! Alteration vs Modulation
//!
//! An 'alteration' is a static, one-time modification of something.
//! A 'modulation' is an alteration + an animation -- an alteration that changes, typically over time
//!

use std::time::Duration;

use crate::animation::*;
use crate::prelude::color_sequence::ColorSequenceAlteration;
use crate::prelude::{Cycle, Extent};

#[derive(Clone)]
pub enum LedEffectAlteration {
  Static(ColorSequenceAlteration),
  Rotating(Rotating),
}

impl LedEffectModulation for LedEffectAlteration {
  fn apply(&mut self, delta: Duration) -> ColorSequenceAlteration {
    match self {
      Self::Static(a) => a.clone(),
      Self::Rotating(r) => r.apply(delta),
    }
  }

  fn reset(&mut self) {
    match self {
      LedEffectAlteration::Static(_) => {}
      LedEffectAlteration::Rotating(r) => r.reset(),
    }
  }
}

pub trait LedEffectModulation {
  fn apply(&mut self, delta: Duration) -> ColorSequenceAlteration;
  fn reset(&mut self);
}

#[derive(Clone)]
pub struct Rotating {
  anim: Tween<Duration, f32>,
}

impl Rotating {
  pub fn new(duration: Duration, curve: Curve) -> Self {
    Self {
      anim: Tween::new(duration, curve, vec![0.0f32, 360.0], Cycle::Forever),
    }
  }
}

impl LedEffectModulation for Rotating {
  fn apply(&mut self, delta: Duration) -> ColorSequenceAlteration {
    self.anim.accumulate(delta);
    ColorSequenceAlteration::Rotate(Extent::Relative(self.anim.sample()))
  }

  fn reset(&mut self) {
    self.anim.reset();
  }
}
