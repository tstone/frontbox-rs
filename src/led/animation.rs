use std::time::Duration;

pub trait Animation<T> {
  /// Returns the remainder, if any
  fn tick(&mut self, delta_time: Duration) -> Duration;
  fn sample(&self) -> T;
  fn is_complete(&self) -> bool;
}

pub enum AnimationCycle {
  Times(u32),
  Forever,
}
