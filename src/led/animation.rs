use dyn_clone::DynClone;
use std::time::Duration;

pub trait Animation<T>: DynClone + Send + Sync {
  /// Returns the remainder, if any
  fn tick(&mut self, delta_time: Duration) -> Duration;
  fn sample(&self) -> T;
  fn is_complete(&self) -> bool;
}

dyn_clone::clone_trait_object!(<T> Animation<T>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimationCycle {
  Times(u32),
  Forever,
}
