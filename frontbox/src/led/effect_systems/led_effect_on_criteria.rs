use std::sync::Arc;

use crate::prelude::*;

#[derive(Clone)]
pub struct LedEffectOnCriteria {
  effects: Vec<LedEffect>,
  criteria: Arc<dyn Fn(&Context) -> bool + 'static>,
}

impl LedEffectOnCriteria {
  /// Create a system that applies an effect if the given criteria is met
  pub fn single<S>(criteria: impl Fn(&Context) -> bool + 'static, effect: LedEffect) -> Self {
    Self {
      effects: vec![effect],
      criteria: Arc::new(criteria),
    }
  }

  pub fn multi(criteria: impl Fn(&Context) -> bool + 'static, effects: Vec<LedEffect>) -> Self {
    Self {
      effects,
      criteria: Arc::new(criteria),
    }
  }
}

impl System for LedEffectOnCriteria {
  fn is_active(&self, ctx: &Context) -> bool {
    (self.criteria)(ctx)
  }

  fn on_tick(&mut self, delta: Duration, ctx: &Context) {
    for effect in &mut self.effects {
      effect.apply(delta, ctx);
    }
  }
}
