//! # Cues
//!
//! Cues are events that a system can send to itself. There are four primitive types of cues:
//!
//! 1. **Once** -- Cue happens exactly once, after a given amount of time has elapsed
//! 2. **Times** -- Cue happens N times, with an interval in between
//! 3. **Forever** -- Cue happens until canceled, with an interval in between
//! 4. **Now** -- Cue happens immediately, once
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! # struct Example;
//! #[derive(Serialize, Event)]
//! struct SomethingImportant(u16);
//!
//! impl System for Example {
//!   fn on_spawn(&mut self, ctx: &SystemContext) {
//!     // setup the cue
//!     ctx.cues.start(
//!       SomethingImportant(100),
//!       Cue::Times(3, Duration::from_secs(3)),
//!     );
//!   }
//!
//!   fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
//!     // what to do when the cue happens (in this case, 3 times)
//!     if let Some(v) = event.downcast_ref::<SomethingImportant>() {
//!       log::debug!("Something important: {}!", v.0);
//!     }
//!   }
//! }
//! ```
//!
//! ### Handles
//!
//! Creating a cue returns a handle that can later be used to cancel it.
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! # #[derive(Serialize, Event)]
//! # struct SomethingRather;
//! # fn example(ctx: &SystemContext) {
//! let handle = ctx.cues.start(SomethingRather, Cue::Forever(Duration::from_secs(1)));
//! // ...
//! ctx.cues.cancel(handle);
//! # }
//! ```
//!
//! ### Cycling
//!
//! Cycling through a set of states is a common occurrence in pinball. For example, flashing is in fact the cycling of two values.
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! # #[derive(Serialize, Event)]
//! # struct On;
//! # #[derive(Serialize, Event)]
//! # struct Off;
//! # fn example(ctx: &SystemContext) {
//! // note the use of `events!` here rather than `vec!`
//! ctx.cues.start_multi(
//!   events![On, Off],
//!   Cue::Forever(Duration::from_secs(1))
//! );
//! # }
//! ```
//!
//! Cycling works by rotating through the list of values each time the cue fires. Think of it like a normal cue that just keeps rotating which signal is emitted, in order. In the example above, 1 second would elapse, then `On` would be cued. Another second would elapse and `Off` would be cued. Another second would elapse and `On` would be cued, and so on.

use std::sync::Arc;
use std::time::Duration;

use crate::prelude::Event;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cue {
  Now,
  Once(Duration),
  Times(u16, Duration),
  Forever(Duration),
}

/// A running cue: how far it is through its interval, how many times it has left to fire, and which of its signals
/// goes next
#[derive(Clone)]
pub(crate) struct CueAccumulator {
  interval: Duration,
  /// Fires left, or `None` for a cue that runs until canceled
  remaining: Option<u16>,
  elapsed: Duration,
  signals: Arc<Vec<Box<dyn Event>>>,
  /// The signal the next fire sends, cycling through `signals`
  next_signal: usize,
}

impl CueAccumulator {
  pub(crate) fn new(cue: Cue, signals: Vec<Box<dyn Event>>) -> Self {
    let (interval, remaining) = match cue {
      Cue::Now => (Duration::ZERO, Some(1)),
      Cue::Once(interval) => (interval, Some(1)),
      Cue::Times(times, interval) => (interval, Some(times)),
      Cue::Forever(interval) => (interval, None),
    };

    Self {
      interval,
      remaining,
      elapsed: Duration::ZERO,
      signals: Arc::new(signals),
      next_signal: 0,
    }
  }

  /// Move the cue forward in time, calling `fire` with the signal for each time it came due, in order. A cue with
  /// a zero interval fires at most once per call.
  pub(crate) fn accumulate(&mut self, delta: Duration, mut fire: impl FnMut(&dyn Event)) {
    self.elapsed += delta;

    while !self.is_complete() && self.elapsed >= self.interval {
      self.elapsed -= self.interval;
      if let Some(remaining) = &mut self.remaining {
        *remaining -= 1;
      }

      if !self.signals.is_empty() {
        fire(self.signals[self.next_signal].as_ref());
        self.next_signal = (self.next_signal + 1) % self.signals.len();
      }

      if self.interval.is_zero() {
        self.elapsed = Duration::ZERO;
        break;
      }
    }
  }

  pub(crate) fn is_complete(&self) -> bool {
    self.remaining == Some(0)
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::{events, prelude::EventExt};

  #[derive(Debug, serde::Serialize, Event, PartialEq, Eq)]
  struct Signal;
  #[derive(Debug, serde::Serialize, Event, PartialEq, Eq)]
  struct Signal2;

  fn ms(ms: u64) -> Duration {
    Duration::from_millis(ms)
  }

  /// Advance `cue` by `delta` and name the signals it fired, in order
  fn advance(cue: &mut CueAccumulator, delta: Duration) -> Vec<&'static str> {
    let mut fired = vec![];
    cue.accumulate(delta, |signal| {
      fired.push(if signal.downcast_ref::<Signal>().is_some() {
        "Signal"
      } else if signal.downcast_ref::<Signal2>().is_some() {
        "Signal2"
      } else {
        "unknown"
      })
    });
    fired
  }

  #[test]
  fn once_fires_when_its_interval_is_up_and_then_never_again() {
    let mut cue = CueAccumulator::new(Cue::Once(ms(1000)), events![Signal]);

    assert!(advance(&mut cue, ms(999)).is_empty());
    assert_eq!(advance(&mut cue, ms(1)), ["Signal"]);
    assert!(cue.is_complete());
    assert!(advance(&mut cue, ms(1000)).is_empty());
  }

  #[test]
  fn once_fires_when_a_tick_overshoots_its_interval() {
    let mut cue = CueAccumulator::new(Cue::Once(ms(1000)), events![Signal]);

    assert_eq!(advance(&mut cue, ms(1100)), ["Signal"]);
    assert!(cue.is_complete());
  }

  #[test]
  fn now_fires_on_the_first_tick_even_with_no_time_passed() {
    let mut cue = CueAccumulator::new(Cue::Now, events![Signal]);

    assert_eq!(advance(&mut cue, Duration::ZERO), ["Signal"]);
    assert!(cue.is_complete());
  }

  #[test]
  fn times_fires_that_many_times_then_completes() {
    let mut cue = CueAccumulator::new(Cue::Times(3, ms(1000)), events![Signal]);

    for _ in 0..2 {
      assert_eq!(advance(&mut cue, ms(1000)), ["Signal"]);
      assert!(!cue.is_complete());
    }
    assert_eq!(advance(&mut cue, ms(1000)), ["Signal"]);
    assert!(cue.is_complete());
    assert!(advance(&mut cue, ms(1000)).is_empty());
  }

  #[test]
  fn zero_times_is_complete_from_the_start() {
    let mut cue = CueAccumulator::new(Cue::Times(0, ms(1000)), events![Signal]);

    assert!(cue.is_complete());
    assert!(advance(&mut cue, ms(1000)).is_empty());
  }

  #[test]
  fn forever_keeps_firing() {
    let mut cue = CueAccumulator::new(Cue::Forever(ms(1000)), events![Signal]);

    for _ in 0..10 {
      assert_eq!(advance(&mut cue, ms(1000)), ["Signal"]);
    }
    assert!(!cue.is_complete());
  }

  #[test]
  fn multiple_signals_cycle_in_order() {
    let mut cue = CueAccumulator::new(Cue::Times(3, ms(1000)), events![Signal, Signal2]);

    assert!(advance(&mut cue, ms(500)).is_empty());
    assert_eq!(advance(&mut cue, ms(500)), ["Signal"]);
    assert_eq!(advance(&mut cue, ms(1000)), ["Signal2"]);
    assert_eq!(advance(&mut cue, ms(1000)), ["Signal"]);
    assert!(cue.is_complete());
  }

  #[test]
  fn a_long_tick_fires_every_interval_that_came_due() {
    let mut cue = CueAccumulator::new(Cue::Forever(ms(100)), events![Signal, Signal2]);

    assert_eq!(advance(&mut cue, ms(350)), ["Signal", "Signal2", "Signal"]);
    // the 50ms left over carries into the next interval
    assert_eq!(advance(&mut cue, ms(50)), ["Signal2"]);
  }

  #[test]
  fn a_long_tick_fires_times_no_more_than_its_count() {
    let mut cue = CueAccumulator::new(Cue::Times(2, ms(100)), events![Signal]);

    assert_eq!(advance(&mut cue, ms(1000)), ["Signal", "Signal"]);
    assert!(cue.is_complete());
  }

  #[test]
  fn a_zero_interval_fires_once_per_tick() {
    let mut cue = CueAccumulator::new(Cue::Forever(Duration::ZERO), events![Signal]);

    assert_eq!(advance(&mut cue, ms(100)), ["Signal"]);
    assert_eq!(advance(&mut cue, Duration::ZERO), ["Signal"]);
  }
}
