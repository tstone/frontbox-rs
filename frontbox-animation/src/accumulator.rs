use std::fmt::Debug;

use dyn_clone::DynClone;

/// Describes something that can move forward in time
pub trait Accumulator<A>: DynClone {
  /// Returns the remainder, if any
  fn accumulate(&mut self, delta: A) -> AccumulationResult<A>;
  /// Set the accumulator at a specific value (e.g. phase shifting)
  fn force(&mut self, current: A);
  fn reset(&mut self);
  /// Returns true if the accumulator has completed all of its cycles
  fn is_complete(&self) -> bool;
}

dyn_clone::clone_trait_object!(<A> Accumulator<A>);

#[derive(Debug)]
pub struct AccumulationResult<A> {
  /// Accumulators can perform multiple cycles (loops). This flag indicates if a cycle was completed during this accumulation step. To check if the entire accumulator has completed all of its cycles, use `.is_complete()` flag.
  pub completed_cycle: bool,
  /// If the accumulator overshot its target, this will contain the excess amount that can be applied to the next cycle or used for other logic.
  pub remainder: A,
}

impl<A> Default for AccumulationResult<A>
where
  A: Default,
{
  fn default() -> Self {
    AccumulationResult {
      completed_cycle: false,
      remainder: A::default(),
    }
  }
}
