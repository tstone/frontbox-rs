use crate::prelude::*;

#[derive(Clone)]
pub struct LedEffectOnSpawn {
  effect: LedEffect,
}

impl LedEffectOnSpawn {
  /// Apply the given LedEffect on spawn
  pub fn new(effect: LedEffect) -> Self {
    Self { effect }
  }
}

impl System for LedEffectOnSpawn {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.effect.apply(Duration::ZERO, ctx)
  }
}
