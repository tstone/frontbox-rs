use std::time::Duration;

pub struct SystemTimer {
  target: Duration,
  accumulated: Duration,
  mode: TimerMode,
}

impl SystemTimer {
  pub fn new(target: Duration, mode: TimerMode) -> Self {
    Self {
      target,
      mode,
      accumulated: Duration::from_secs(0),
    }
  }

  pub fn tick(&mut self, delta: &Duration) -> bool {
    self.accumulated += *delta;
    if self.accumulated >= self.target {
      self.accumulated = self.accumulated - self.target;
      true
    } else {
      false
    }
  }

  pub fn mode(&self) -> &TimerMode {
    &self.mode
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerMode {
  OneShot,
  Repeating,
}
