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
//! Cycling works by rotating through the list of values each time the cue is complete. Think of it like a normal cue that just keeps rotating which signal is emitted, in order. In the example above, 1 second would elapse, then `On` would be cued. Another second would elapse and `Off` would be cued. Another second would elapse and `On` would be cued, and so on.

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

#[derive(Clone)]
pub(crate) enum CueInternal {
  Once(Duration),
  Times(u16, Duration),
  Loop(Duration),
}

#[derive(Clone)]
pub struct CueAccumulator {
  cue: CueInternal,
  elapsed: Duration,
  signal: Arc<Vec<Box<dyn Event>>>,
  cycle_index: usize,
  loop_count: u16,
  completed: bool,
}

impl CueAccumulator {
  pub fn from_cue(cue: Cue, signals: Vec<Box<dyn Event>>) -> Self {
    let cue_internal = match cue {
      Cue::Now => CueInternal::Once(Duration::ZERO),
      Cue::Once(duration) => CueInternal::Once(duration),
      Cue::Times(t, duration) => CueInternal::Times(t, duration),
      Cue::Forever(duration) => CueInternal::Loop(duration),
    };
    Self::new(cue_internal, signals)
  }

  fn new(cue: CueInternal, signals: Vec<Box<dyn Event>>) -> Self {
    // start with index at end of signals so that first increment will roll it over to 0
    let cycle_index = signals.iter().len();

    Self {
      cue,
      elapsed: Duration::ZERO,
      cycle_index,
      signal: Arc::new(signals),
      loop_count: 0,
      completed: false,
    }
  }

  /// Get the current signal to trigger for this cue
  pub fn signal(&self) -> Option<&dyn Event> {
    if self.cycle_index >= self.signal.len() {
      return None;
    }
    Some(&*self.signal[self.cycle_index].as_ref())
  }

  pub fn target(&self) -> Duration {
    match &self.cue {
      CueInternal::Once(duration) => *duration,
      CueInternal::Times(_, duration) => *duration,
      CueInternal::Loop(duration) => *duration,
    }
  }

  fn increment_signal_index(&mut self) {
    self.cycle_index += 1;
    if self.cycle_index >= self.signal.len() {
      self.cycle_index = 0;
    }
  }

  /// Move the cue forward in time, returning whether it completed a cycle (and so should fire its signal)
  pub fn accumulate(&mut self, delta: Duration) -> bool {
    if self.is_complete() {
      return false;
    }

    // If we're exactly at the target, we need to roll back to 0 before adding delta
    // This can happen when the cycle completes and we don't reset to 0 immediately (see below)
    if self.elapsed == self.target() {
      self.elapsed -= self.target();
    }

    self.elapsed += delta;

    if self.elapsed >= self.target() {
      // don't reset to 0 if exactly at the target. This will happen the next cycle but doing so too early
      // makes `sample` incorrect for the last frame of the cycle. Since once never cycles, don't reset elapsed
      // in that case either.
      let loops = !matches!(self.cue, CueInternal::Once(_));
      if self.elapsed > self.target() && loops {
        self.elapsed -= self.target();
      }

      self.increment_signal_index();

      if matches!(self.cue, CueInternal::Once(_)) {
        // for Once cues, keep track of if the cue fires
        // this avoids an edge case of Duration::ZERO never firing
        self.completed = true;
      }

      match self.cue {
        CueInternal::Times(_, _) => {
          self.loop_count += 1;
        }
        _ => {}
      }

      return true;
    }

    false
  }

  pub fn is_complete(&self) -> bool {
    match &self.cue {
      CueInternal::Once(_) => self.completed,
      CueInternal::Times(t, _) => self.loop_count >= *t,
      CueInternal::Loop(_) => false,
    }
  }
}

#[cfg(test)]
mod test {
  use crate::{events, prelude::EventExt};

  #[derive(Debug, serde::Serialize, Event, PartialEq, Eq)]
  struct Signal;
  #[derive(Debug, serde::Serialize, Event, PartialEq, Eq)]
  struct Signal2;

  pub use super::*;

  #[test]
  fn once_cue() {
    let mut cue = CueAccumulator::new(
      CueInternal::Once(Duration::from_secs(1)),
      vec![Box::new(Signal)],
    );

    assert_eq!(cue.is_complete(), false);
    assert_eq!(cue.signal().is_none(), true);

    // Advance 1 second, should trigger signal
    let completed_cycle = cue.accumulate(Duration::from_secs(1));
    assert!(completed_cycle);
    assert_eq!(cue.is_complete(), true);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal>()),
      Some(&Signal)
    );

    // Verify that accumulating more time doesn't change the state
    let completed_cycle = cue.accumulate(Duration::from_secs(1));
    assert!(!completed_cycle);
    assert_eq!(cue.is_complete(), true);
  }

  /// A Once cue that goes over the exact accumulated time
  #[test]
  fn once_over_cue() {
    let mut cue = CueAccumulator::new(
      CueInternal::Once(Duration::from_secs(1)),
      vec![Box::new(Signal)],
    );

    assert_eq!(cue.is_complete(), false);
    assert_eq!(cue.signal().is_none(), true);

    // Advance 1 second, should trigger signal
    let completed_cycle = cue.accumulate(Duration::from_millis(1100));
    assert!(completed_cycle);
    assert_eq!(cue.is_complete(), true);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal>()),
      Some(&Signal)
    );

    // Verify that accumulating more time doesn't change the state
    let completed_cycle = cue.accumulate(Duration::from_secs(1));
    assert!(!completed_cycle);
    assert_eq!(cue.is_complete(), true);
  }

  #[test]
  fn times_cue() {
    let mut cue = CueAccumulator::new(
      CueInternal::Times(3, Duration::from_secs(1)),
      vec![Box::new(Signal)],
    );

    assert_eq!(cue.is_complete(), false);
    assert_eq!(cue.signal().is_none(), true);

    // Advance 1 second, should trigger first signal
    let completed_cycle = cue.accumulate(Duration::from_secs(1));
    assert!(completed_cycle);
    assert_eq!(cue.is_complete(), false);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal>()),
      Some(&Signal)
    );

    // Advance another second, should trigger second signal
    let completed_cycle = cue.accumulate(Duration::from_secs(1));
    assert!(completed_cycle);
    assert_eq!(cue.is_complete(), false);
    assert_eq!(cue.signal().is_some(), true);

    // Advance another second, should trigger third signal and complete
    let completed_cycle = cue.accumulate(Duration::from_secs(1));
    assert!(completed_cycle);
    assert_eq!(cue.is_complete(), true);
    assert_eq!(cue.signal().is_some(), true);
  }

  #[test]
  fn loop_cue() {
    let mut cue = CueAccumulator::new(
      CueInternal::Loop(Duration::from_secs(1)),
      vec![Box::new(Signal)],
    );

    assert_eq!(cue.is_complete(), false);
    assert_eq!(cue.signal().is_none(), true);

    // Advance 1 second, should trigger first signal
    let completed_cycle = cue.accumulate(Duration::from_secs(1));
    assert!(completed_cycle);
    assert_eq!(cue.is_complete(), false);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal>()),
      Some(&Signal)
    );

    // Advance another second, should trigger second signal
    let completed_cycle = cue.accumulate(Duration::from_secs(1));
    assert!(completed_cycle);
    assert_eq!(cue.is_complete(), false);
    assert_eq!(cue.signal().is_some(), true);
  }

  #[test]
  fn times_cue_with_multiple_signals() {
    let mut cue = CueAccumulator::new(
      CueInternal::Times(3, Duration::from_secs(1)),
      events![Signal, Signal2],
    );

    // Advance 1 second, should trigger first signal
    let completed_cycle = cue.accumulate(Duration::from_millis(500));
    assert!(!completed_cycle);
    let completed_cycle = cue.accumulate(Duration::from_millis(500));
    assert!(completed_cycle);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal>()),
      Some(&Signal)
    );

    // Advance another second, should trigger second signal
    let completed_cycle = cue.accumulate(Duration::from_secs(1));
    assert!(completed_cycle);
    assert_eq!(cue.is_complete(), false);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal2>()),
      Some(&Signal2)
    );

    // Advance another second, should loop back around to the first signal
    let completed_cycle = cue.accumulate(Duration::from_secs(1));
    assert!(completed_cycle);
    assert_eq!(cue.is_complete(), true);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal>()),
      Some(&Signal)
    );
  }
}
