use std::time::Duration;

use dyn_clone::DynClone;

/// Describes something that can move forward in time
pub trait Tickable: DynClone + Send + Sync {
  /// Returns the remainder, if any
  fn tick(&mut self, delta: Duration) -> Duration;
  fn reset(&mut self);
  fn is_complete(&self) -> bool;
  fn completed_this_tick(&self) -> bool;
}

dyn_clone::clone_trait_object!(Tickable);
