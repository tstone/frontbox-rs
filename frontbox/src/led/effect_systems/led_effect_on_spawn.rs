use crate::prelude::*;

#[derive(Clone)]
pub struct LedEffectOnSpawn<S: ColorSequence + Clone + Send + Sync + 'static> {
  effect: LedEffect<S>,
}

impl<S> LedEffectOnSpawn<S>
where
  S: ColorSequence + Clone + Send + Sync + 'static,
{
  /// Apply the given LedEffect on spawn
  pub fn new(effect: LedEffect<S>) -> Self {
    Self { effect }
  }
}

impl<S> System for LedEffectOnSpawn<S>
where
  S: ColorSequence + Clone + Send + Sync + 'static,
{
  fn on_spawn(&mut self, ctx: &Context) {
    self.effect.apply(Duration::ZERO, ctx)
  }
}
