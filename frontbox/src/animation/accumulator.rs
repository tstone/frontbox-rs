use dyn_clone::DynClone;

/// Describes something that can move forward in time
pub trait Accumulator<A>: DynClone + Send + Sync {
  /// Returns the remainder, if any
  fn accumulate(&mut self, delta: A) -> AccumulationResult<A>;
  /// Set the accumulator at a specific value
  fn set(&mut self, current: A);
  fn reset(&mut self);
  fn is_complete(&self) -> bool;
}

dyn_clone::clone_trait_object!(<A> Accumulator<A>);

pub struct AccumulationResult<A> {
  pub completed_just_now: bool,
  pub remainder: A,
}

impl<A> Default for AccumulationResult<A>
where
  A: Default,
{
  fn default() -> Self {
    AccumulationResult {
      completed_just_now: false,
      remainder: A::default(),
    }
  }
}
