use std::sync::Arc;

use crate::prelude::*;

#[derive(Clone)]
pub struct LedEffectOnCriteria<S: ColorSequence + Clone + Send + Sync + 'static> {
  effect: LedEffect<S>,
  criteria: Arc<dyn Fn(&Context) -> bool + 'static>,
}

impl<S> LedEffectOnCriteria<S>
where
  S: ColorSequence + Clone + Send + Sync + 'static,
{
  /// Create a system that applies an effect if the given criteria is met
  pub fn new(criteria: impl Fn(&Context) -> bool + 'static, effect: LedEffect<S>) -> Self {
    Self {
      effect,
      criteria: Arc::new(criteria),
    }
  }
}

impl<S> System for LedEffectOnCriteria<S>
where
  S: ColorSequence + Clone + Send + Sync + 'static,
{
  fn is_active(&self, ctx: &Context) -> bool {
    (self.criteria)(ctx)
  }

  fn on_tick(&mut self, delta: Duration, ctx: &Context) {
    self.effect.apply(delta, ctx);
  }
}
