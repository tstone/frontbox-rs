//! # Timers
//!
//! A timer sends an event back to the system that started it once its time has passed. Only that system hears it.
//! There are four kinds of timer:
//!
//! 1. **Once** -- fires exactly once, after a given amount of time has elapsed
//! 2. **Times** -- fires N times, with an interval in between
//! 3. **Forever** -- fires until canceled, with an interval in between
//! 4. **Now** -- fires once, immediately
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! # struct Example;
//! #[derive(Serialize, Event)]
//! struct SomethingImportant(u16);
//!
//! impl System for Example {
//!   fn on_spawn(&mut self, ctx: &SystemContext) {
//!     // setup the timer
//!     ctx.timers.start(
//!       SomethingImportant(100),
//!       Timer::Times(3, Duration::from_secs(3)),
//!     );
//!   }
//!
//!   fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
//!     // what to do when the timer fires (in this case, 3 times)
//!     if let Some(v) = event.downcast_ref::<SomethingImportant>() {
//!       log::debug!("Something important: {}!", v.0);
//!     }
//!   }
//! }
//! ```
//!
//! ### Handles
//!
//! Creating a timer returns a handle that can later be used to cancel it.
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! # #[derive(Serialize, Event)]
//! # struct SomethingRather;
//! # fn example(ctx: &SystemContext) {
//! let handle = ctx.timers.start(SomethingRather, Timer::Forever(Duration::from_secs(1)));
//! // ...
//! ctx.timers.cancel(handle);
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
//! ctx.timers.start_multi(
//!   events![On, Off],
//!   Timer::Forever(Duration::from_secs(1))
//! );
//! # }
//! ```
//!
//! Cycling works by rotating through the list of values each time the timer fires. Think of it like a normal timer that just keeps rotating which signal is emitted, in order. In the example above, 1 second would elapse, then `On` would be sent. Another second would elapse and `Off` would be sent. Another second would elapse and `On` would be sent, and so on.

//! ### Pausing
//!
//! A timer can be paused with its handle, and resumed later. While paused, time doesn't count toward it: a
//! `Timer::Once(30s)` paused 10 seconds in fires 20 seconds after it's resumed. A paused timer can still be canceled.
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! # #[derive(Serialize, Event)]
//! # struct TimeUp;
//! # fn example(ctx: &SystemContext) {
//! let handle = ctx.timers.start(TimeUp, Timer::Once(Duration::from_secs(30)));
//! ctx.timers.pause(handle);
//! // ...
//! ctx.timers.resume(handle);
//! # }
//! ```
//!
//! A system's timers also stop counting while the system is inactive, as it isn't ticked.

use std::sync::Arc;
use std::time::Duration;

use crate::prelude::Event;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Timer {
  Now,
  Once(Duration),
  Times(u16, Duration),
  Forever(Duration),
}

/// A running timer: how far it is through its interval, how many times it has left to fire, and which of its signals
/// goes next
#[derive(Clone)]
pub(crate) struct TimerAccumulator {
  interval: Duration,
  /// Fires left, or `None` for a timer that runs until canceled
  remaining: Option<u16>,
  elapsed: Duration,
  signals: Arc<Vec<Box<dyn Event>>>,
  /// The signal the next fire sends, cycling through `signals`
  next_signal: usize,
  /// While paused, time passing doesn't count toward the timer
  paused: bool,
}

impl TimerAccumulator {
  pub(crate) fn new(timer: Timer, signals: Vec<Box<dyn Event>>) -> Self {
    let (interval, remaining) = match timer {
      Timer::Now => (Duration::ZERO, Some(1)),
      Timer::Once(interval) => (interval, Some(1)),
      Timer::Times(times, interval) => (interval, Some(times)),
      Timer::Forever(interval) => (interval, None),
    };

    Self {
      interval,
      remaining,
      elapsed: Duration::ZERO,
      signals: Arc::new(signals),
      next_signal: 0,
      paused: false,
    }
  }

  pub(crate) fn set_paused(&mut self, paused: bool) {
    self.paused = paused;
  }

  /// Move the timer forward in time, calling `fire` with the signal for each time it came due, in order. A timer with
  /// a zero interval fires at most once per call.
  /// A paused timer doesn't move, and fires nothing.
  pub(crate) fn accumulate(&mut self, delta: Duration, mut fire: impl FnMut(&dyn Event)) {
    if self.paused {
      return;
    }
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

  /// Advance `timer` by `delta` and name the signals it fired, in order
  fn advance(timer: &mut TimerAccumulator, delta: Duration) -> Vec<&'static str> {
    let mut fired = vec![];
    timer.accumulate(delta, |signal| {
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
    let mut timer = TimerAccumulator::new(Timer::Once(ms(1000)), events![Signal]);

    assert!(advance(&mut timer, ms(999)).is_empty());
    assert_eq!(advance(&mut timer, ms(1)), ["Signal"]);
    assert!(timer.is_complete());
    assert!(advance(&mut timer, ms(1000)).is_empty());
  }

  #[test]
  fn once_fires_when_a_tick_overshoots_its_interval() {
    let mut timer = TimerAccumulator::new(Timer::Once(ms(1000)), events![Signal]);

    assert_eq!(advance(&mut timer, ms(1100)), ["Signal"]);
    assert!(timer.is_complete());
  }

  #[test]
  fn now_fires_on_the_first_tick_even_with_no_time_passed() {
    let mut timer = TimerAccumulator::new(Timer::Now, events![Signal]);

    assert_eq!(advance(&mut timer, Duration::ZERO), ["Signal"]);
    assert!(timer.is_complete());
  }

  #[test]
  fn times_fires_that_many_times_then_completes() {
    let mut timer = TimerAccumulator::new(Timer::Times(3, ms(1000)), events![Signal]);

    for _ in 0..2 {
      assert_eq!(advance(&mut timer, ms(1000)), ["Signal"]);
      assert!(!timer.is_complete());
    }
    assert_eq!(advance(&mut timer, ms(1000)), ["Signal"]);
    assert!(timer.is_complete());
    assert!(advance(&mut timer, ms(1000)).is_empty());
  }

  #[test]
  fn zero_times_is_complete_from_the_start() {
    let mut timer = TimerAccumulator::new(Timer::Times(0, ms(1000)), events![Signal]);

    assert!(timer.is_complete());
    assert!(advance(&mut timer, ms(1000)).is_empty());
  }

  #[test]
  fn forever_keeps_firing() {
    let mut timer = TimerAccumulator::new(Timer::Forever(ms(1000)), events![Signal]);

    for _ in 0..10 {
      assert_eq!(advance(&mut timer, ms(1000)), ["Signal"]);
    }
    assert!(!timer.is_complete());
  }

  #[test]
  fn multiple_signals_cycle_in_order() {
    let mut timer = TimerAccumulator::new(Timer::Times(3, ms(1000)), events![Signal, Signal2]);

    assert!(advance(&mut timer, ms(500)).is_empty());
    assert_eq!(advance(&mut timer, ms(500)), ["Signal"]);
    assert_eq!(advance(&mut timer, ms(1000)), ["Signal2"]);
    assert_eq!(advance(&mut timer, ms(1000)), ["Signal"]);
    assert!(timer.is_complete());
  }

  #[test]
  fn a_long_tick_fires_every_interval_that_came_due() {
    let mut timer = TimerAccumulator::new(Timer::Forever(ms(100)), events![Signal, Signal2]);

    assert_eq!(
      advance(&mut timer, ms(350)),
      ["Signal", "Signal2", "Signal"]
    );
    // the 50ms left over carries into the next interval
    assert_eq!(advance(&mut timer, ms(50)), ["Signal2"]);
  }

  #[test]
  fn a_long_tick_fires_times_no_more_than_its_count() {
    let mut timer = TimerAccumulator::new(Timer::Times(2, ms(100)), events![Signal]);

    assert_eq!(advance(&mut timer, ms(1000)), ["Signal", "Signal"]);
    assert!(timer.is_complete());
  }

  #[test]
  fn a_paused_timer_holds_its_progress_until_resumed() {
    let mut timer = TimerAccumulator::new(Timer::Once(ms(1000)), events![Signal]);

    assert!(advance(&mut timer, ms(600)).is_empty());
    timer.set_paused(true);
    assert!(advance(&mut timer, ms(5000)).is_empty());
    assert!(!timer.is_complete());

    timer.set_paused(false);
    assert!(advance(&mut timer, ms(399)).is_empty());
    assert_eq!(advance(&mut timer, ms(1)), ["Signal"]);
  }

  #[test]
  fn a_paused_now_timer_waits_for_resume() {
    let mut timer = TimerAccumulator::new(Timer::Now, events![Signal]);
    timer.set_paused(true);

    assert!(advance(&mut timer, Duration::ZERO).is_empty());
    timer.set_paused(false);
    assert_eq!(advance(&mut timer, Duration::ZERO), ["Signal"]);
  }

  #[test]
  fn a_paused_cycling_timer_resumes_on_its_next_signal() {
    let mut timer = TimerAccumulator::new(Timer::Forever(ms(100)), events![Signal, Signal2]);

    assert_eq!(advance(&mut timer, ms(100)), ["Signal"]);
    timer.set_paused(true);
    assert!(advance(&mut timer, ms(1000)).is_empty());
    timer.set_paused(false);
    assert_eq!(advance(&mut timer, ms(100)), ["Signal2"]);
  }

  #[test]
  fn a_zero_interval_fires_once_per_tick() {
    let mut timer = TimerAccumulator::new(Timer::Forever(Duration::ZERO), events![Signal]);

    assert_eq!(advance(&mut timer, ms(100)), ["Signal"]);
    assert_eq!(advance(&mut timer, Duration::ZERO), ["Signal"]);
  }
}
