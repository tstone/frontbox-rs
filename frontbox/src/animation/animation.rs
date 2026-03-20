use std::time::Duration;

use dyn_clone::DynClone;

/// Describes any value that can be changed over time
pub trait Animation<T>: DynClone + Send + Sync {
  /// Returns the remainder, if any
  fn tick(&mut self, delta: Duration) -> Duration;
  fn sample(&self) -> T;
  fn is_complete(&self) -> bool;
  fn reset(&mut self);
}

dyn_clone::clone_trait_object!(<T> Animation<T>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimationCycle {
  Once,
  Times(u32),
  Forever,
}
