use std::sync::Arc;

use crate::prelude::*;

#[derive(Clone)]
pub struct LedEffectOnCriteria {
  effects: Vec<Box<dyn DynLedEffect>>,
  criteria: Arc<dyn Fn(&Context) -> bool + 'static>,
}

impl LedEffectOnCriteria {
  /// Create a system that applies an effect if the given criteria is met
  pub fn single<S>(criteria: impl Fn(&Context) -> bool + 'static, effect: LedEffect<S>) -> Self
  where
    S: ColorSequence + Clone + Send + Sync + 'static,
  {
    Self {
      effects: vec![Box::new(effect)],
      criteria: Arc::new(criteria),
    }
  }

  pub fn multi(
    criteria: impl Fn(&Context) -> bool + 'static,
    effects: Vec<Box<dyn DynLedEffect>>,
  ) -> Self {
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
