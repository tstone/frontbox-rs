use std::sync::Arc;
use std::time::Duration;

use crate::animation::*;
use crate::prelude::Signal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cue {
  Now,
  Once(Duration),
  Times(u16, Duration),
  Loop(Duration),
}

#[derive(Clone)]
pub struct CueAccumulator {
  cue: Cue,
  elapsed: Duration,
  signal: Arc<Vec<Box<dyn Signal>>>,
  signal_index: usize,
  loop_count: u16,
}

impl CueAccumulator {
  pub fn new(cue: Cue, signals: Vec<Box<dyn Signal>>) -> Self {
    let signal_index = match cue {
      Cue::Now => 0,
      // start with index at end of signals so that first increment will roll it over to 0
      _ => signals.iter().len(),
    };

    Self {
      cue,
      elapsed: Duration::ZERO,
      signal_index,
      signal: Arc::new(signals),
      loop_count: 0,
    }
  }

  /// Get the current signal to trigger for this cue
  pub fn signal(&self) -> Option<&dyn Signal> {
    if self.signal_index >= self.signal.len() {
      return None;
    }
    Some(&*self.signal[self.signal_index].as_ref())
  }

  pub fn target(&self) -> Duration {
    match self.cue {
      Cue::Now => Duration::ZERO,
      Cue::Once(duration) => duration,
      Cue::Times(_, duration) => duration,
      Cue::Loop(duration) => duration,
    }
  }

  fn increment_signal_index(&mut self) {
    self.signal_index += 1;
    if self.signal_index >= self.signal.len() {
      self.signal_index = 0;
    }
  }
}

impl Accumulator<Duration> for CueAccumulator {
  fn accumulate(&mut self, delta: Duration) -> AccumulationResult<Duration> {
    let mut result = AccumulationResult {
      remainder: Duration::ZERO,
      completed_cycle: false,
    };

    if self.cue == Cue::Now || self.is_complete() {
      return result;
    }

    // If we're exactly at the target, we need to roll back to 0 before adding delta
    // This can happen when the cycle completes and we don't reset to 0 immediately (see below)
    if self.elapsed == self.target() {
      self.elapsed -= self.target();
    }

    self.elapsed += delta;

    if self.elapsed >= self.target() {
      // don't reset to 0 if exactly at the target. This will happen the next cycle but doing so too early
      // makes `sample` incorrect for the last frame of the cycle
      if self.elapsed > self.target() {
        self.elapsed -= self.target();
      }

      result.completed_cycle = true;
      result.remainder = self.elapsed;
      self.increment_signal_index();

      match self.cue {
        Cue::Times(_, _) => {
          self.loop_count += 1;
        }
        _ => {}
      }
    }

    result
  }

  fn set(&mut self, elapsed: Duration) {
    self.elapsed = elapsed;
  }

  fn reset(&mut self) {
    self.elapsed = Duration::ZERO;
  }

  fn is_complete(&self) -> bool {
    match self.cue {
      Cue::Now => true,
      Cue::Once(duration) => self.elapsed >= duration,
      Cue::Times(t, _) => self.loop_count >= t,
      Cue::Loop(_) => false,
    }
  }
}

#[cfg(test)]
mod test {
  use crate::{events, prelude::SignalExt};

  pub use super::*;

  #[test]
  fn now_cue() {
    let cue = CueAccumulator::new(Cue::Now, vec![Box::new("signal")]);

    assert_eq!(cue.is_complete(), true);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<&str>()),
      Some(&"signal")
    );
  }

  #[test]
  fn once_cue() {
    let mut cue = CueAccumulator::new(Cue::Once(Duration::from_secs(1)), vec![Box::new("signal")]);

    assert_eq!(cue.is_complete(), false);
    assert_eq!(cue.signal().is_none(), true);

    // Advance 1 second, should trigger signal
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(cue.is_complete(), true);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<&str>()),
      Some(&"signal")
    );
  }

  #[test]
  fn times_cue() {
    let mut cue = CueAccumulator::new(
      Cue::Times(3, Duration::from_secs(1)),
      vec![Box::new("signal")],
    );

    assert_eq!(cue.is_complete(), false);
    assert_eq!(cue.signal().is_none(), true);

    // Advance 1 second, should trigger first signal
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(cue.is_complete(), false);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<&str>()),
      Some(&"signal")
    );

    // Advance another second, should trigger second signal
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(cue.is_complete(), false);
    assert_eq!(cue.signal().is_some(), true);

    // Advance another second, should trigger third signal and complete
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(cue.is_complete(), true);
    assert_eq!(cue.signal().is_some(), true);
  }

  #[test]
  fn loop_cue() {
    let mut cue = CueAccumulator::new(Cue::Loop(Duration::from_secs(1)), vec![Box::new("signal")]);

    assert_eq!(cue.is_complete(), false);
    assert_eq!(cue.signal().is_none(), true);

    // Advance 1 second, should trigger first signal
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(cue.is_complete(), false);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<&str>()),
      Some(&"signal")
    );

    // Advance another second, should trigger second signal
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(cue.is_complete(), false);
    assert_eq!(cue.signal().is_some(), true);
  }

  #[test]
  fn times_cue_with_multiple_signals() {
    let mut cue = CueAccumulator::new(
      Cue::Times(3, Duration::from_secs(1)),
      events!["signal1", "signal2"],
    );

    // Advance 1 second, should trigger first signal
    let result = cue.accumulate(Duration::from_millis(500));
    assert_eq!(result.completed_cycle, false);
    let result = cue.accumulate(Duration::from_millis(500));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<&str>()),
      Some(&"signal1")
    );

    // Advance another second, should trigger second signal
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(cue.is_complete(), false);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<&str>()),
      Some(&"signal2")
    );

    // Advance another second, should loop back around to the first signal
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(cue.is_complete(), true);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<&str>()),
      Some(&"signal1")
    );
  }
}
