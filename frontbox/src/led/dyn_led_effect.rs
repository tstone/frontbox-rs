use crate::prelude::*;
use dyn_clone::{DynClone, clone_trait_object};
use std::time::Duration;

pub trait DynLedEffect: DynClone + Send + Sync {
  fn apply(&mut self, delta: Duration, ctx: &Context);
}
clone_trait_object!(DynLedEffect);

impl<S> DynLedEffect for LedEffect<S>
where
  S: ColorSequence + Clone + Send + Sync + 'static,
{
  fn apply(&mut self, delta: Duration, ctx: &Context) {
    LedEffect::apply(self, delta, ctx);
  }
}
