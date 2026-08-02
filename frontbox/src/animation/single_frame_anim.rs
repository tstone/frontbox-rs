use crate::animation::{AccumulationResult, Accumulator, Animation};

#[derive(Clone)]
pub struct SingleFrameAnim<T: Clone + Send + Sync> {
  value: T,
}

impl<T> SingleFrameAnim<T>
where
  T: Clone + Send + Sync,
{
  pub fn new(value: T) -> Self {
    Self { value }
  }
}

impl<A, T> Animation<A, T> for SingleFrameAnim<T>
where
  T: Clone + Send + Sync,
  A: Default,
{
  fn pause(&mut self) {}

  fn play(&mut self) {}

  fn sample(&self) -> T {
    self.value.clone()
  }

  fn stop(&mut self) {}
}

impl<A, T> Accumulator<A> for SingleFrameAnim<T>
where
  T: Clone + Send + Sync,
  A: Default,
{
  fn accumulate(&mut self, _delta: A) -> AccumulationResult<A> {
    AccumulationResult {
      ..Default::default()
    }
  }

  fn force(&mut self, _current: A) {}

  fn is_complete(&self) -> bool {
    false
  }

  fn reset(&mut self) {}
}
