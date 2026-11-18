use crate::prelude::*;

pub trait TimerExt {
  fn once(&self) -> Timer;
  fn times(&self, n: u16) -> Timer;
  fn forever(&self) -> Timer;
}

impl TimerExt for Duration {
  fn once(&self) -> Timer {
    Timer::Once(*self)
  }

  fn times(&self, n: u16) -> Timer {
    Timer::Times(n, *self)
  }

  fn forever(&self) -> Timer {
    Timer::Forever(*self)
  }
}
