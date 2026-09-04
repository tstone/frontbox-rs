//! # Cues
//!
//! Cues are events that a system can send to itself. There are four primitive types of cues:
//!
//! 1. **Once** -- Cue happens exactly once, after a given amount of time has elapsed
//! 2. **Times** -- Cue happens N times, with an interval in between
//! 3. **Loop** -- Cue happens until canceled, with an interval in between
//! 4. **Now** -- Cue happens immediately, once
//!
//! ```rust
//! struct SomethingImportant(u8);
//!
//! impl System for Example {
//!   fn on_startup(&mut self, ctx: &Context) {
//!     // setup the cue
//!     ctx.cue(
//!       SomethingImportant(100),
//!       Cue::Times(3, Duration::from_secs(3)),
//!     )
//!   }
//!
//!   fn on_event(&mut self, event: &dyn Signal, ctx: &mut Context) {
//!     // what to do when the cue happens (in this case, 3 times)
//!     if let Some(v) = cmd.downcast_ref::<SomethingImportant>() {
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
//! let handle = ctx.cue(SomethingRather, Cue::Forever(Duration::from_secs(1)));
//! // ...
//! ctx.cancel_cue(handle);
//! ```
//!
//! ### Cycling
//!
//! Cycling through a set of states is a common occurrence in pinball. For example, flashing is in fact the cycling of two values.
//!
//! ```rust
//! // note the use of `events!` here rather than `vec!`
//! ctx.cue_cycling(
//!   events![On, Off],
//!   Cue::Forever(Duration::from_secs(1))
//! );
//! ```
//!
//! Cycling works by rotating through the list of values each time the cue is complete. Think of it like a normal cue that just keeps rotating which signal is emitted, in order. In the example above, 1 second would elapse, then `On("example")` would be cued. Another second would elapse and `Off("example")` would be cued. Another second would elapse and `On("example")` would be cued, and so on.
//!
//! ### Timelines
//!
//! Sometimes it's easier to express things as a linear timeline. The same example as above could also be expressed as...
//!
//! ```rust
//! ctx.cue_timeline(CueTimeline::new()
//!   .cue_at(Duration::from_secs(3), SomethingImportant(5))
//!   .cue_at(Duration::from_millis(4150), SomethingImportant(50))
//!   .cue_at(Duration::from_secs(9), SomethingImportant(500))
//! );
//! ```
//!
//! Timelines are a way to group a bunch of one-off cues into a sequence. Canceling a timeline cancels all remaining cues within it.
//!

use std::sync::Arc;
use std::time::Duration;

use itertools::Itertools;

use crate::animation::*;
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
  Timeline(Vec<CueAccumulator>),
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

  pub fn from_points(points: Vec<(Duration, Box<dyn Event>)>) -> Self {
    // assert that there are no duplicated durations since two points cannot resolve at the same time
    let mut seen_durations = std::collections::HashSet::new();
    for (duration, _) in &points {
      if !seen_durations.insert(*duration) {
        panic!(
          "Duplicate duration {} in timeline cue",
          duration.as_secs_f32()
        );
      }
    }

    let cue_points = points
      .into_iter()
      // sort by least duration to greatest so that the timeline can process them in order
      .sorted_by_key(|(target, _)| *target)
      .map(|(time, event)| CueAccumulator::new(CueInternal::Once(time), vec![event]))
      .collect();
    Self::new(CueInternal::Timeline(cue_points), vec![])
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
    match &self.cue {
      CueInternal::Timeline(points) => {
        // only check the first point since only one point can be complete at a time
        if let Some(first_point) = points.first() {
          if first_point.is_complete() {
            return first_point.signal();
          }
        }
        None
      }
      _ => {
        // default: use the signal index (cycling)
        if self.cycle_index >= self.signal.len() {
          return None;
        }
        Some(&*self.signal[self.cycle_index].as_ref())
      }
    }
  }

  pub fn target(&self) -> Duration {
    match &self.cue {
      CueInternal::Once(duration) => *duration,
      CueInternal::Times(_, duration) => *duration,
      CueInternal::Loop(duration) => *duration,
      CueInternal::Timeline(points) => {
        // timeline target is the duration of the longest point
        points
          .iter()
          .map(|point| point.target())
          .max()
          .unwrap_or(Duration::ZERO)
      }
    }
  }

  fn increment_signal_index(&mut self) {
    self.cycle_index += 1;
    if self.cycle_index >= self.signal.len() {
      self.cycle_index = 0;
    }
  }
}

impl Accumulator<Duration> for CueAccumulator {
  fn accumulate(&mut self, delta: Duration) -> AccumulationResult<Duration> {
    let mut result = AccumulationResult {
      remainder: Duration::ZERO,
      completed_cycle: false,
    };

    if self.is_complete() {
      return result;
    }

    // If we're exactly at the target, we need to roll back to 0 before adding delta
    // This can happen when the cycle completes and we don't reset to 0 immediately (see below)
    if self.elapsed == self.target() {
      self.elapsed -= self.target();
    }

    self.elapsed += delta;

    match &mut self.cue {
      CueInternal::Timeline(points) => {
        // check if the first point has completed, and remove it if so
        if let Some(first) = points.first() {
          if first.is_complete() {
            points.remove(0);
          }
        }

        // save the result of the (now) first point as that will be the return of this function
        let first_result = points.first_mut().map(|point| point.accumulate(delta));

        // accumulate time to remaining points so they are ready to trigger when their time comes
        for point in points.iter_mut().skip(1) {
          point.accumulate(delta);
        }

        if let Some(result) = first_result {
          return result;
        }
      }
      _ => {}
    }

    if self.elapsed >= self.target() {
      // don't reset to 0 if exactly at the target. This will happen the next cycle but doing so too early
      // makes `sample` incorrect for the last frame of the cycle. Since once never cycles, don't reset elapsed
      // in that case either.
      let loops = !matches!(self.cue, CueInternal::Once(_));
      if self.elapsed > self.target() && loops {
        self.elapsed -= self.target();
      }

      result.completed_cycle = true;
      result.remainder = self.elapsed;
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
    }

    result
  }

  fn force(&mut self, elapsed: Duration) {
    self.elapsed = elapsed;
  }

  fn reset(&mut self) {
    self.elapsed = Duration::ZERO;
  }

  fn is_complete(&self) -> bool {
    match &self.cue {
      CueInternal::Once(_) => self.completed,
      CueInternal::Times(t, _) => self.loop_count >= *t,
      CueInternal::Loop(_) => false,
      CueInternal::Timeline(points) => {
        points.len() == 0 || (points.len() == 1 && points[0].is_complete())
      }
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
  #[derive(Debug, serde::Serialize, Event, PartialEq, Eq)]
  struct Signal3;

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
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(cue.is_complete(), true);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal>()),
      Some(&Signal)
    );

    // Verify that accumulating more time doesn't change the state
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, false);
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
    let result = cue.accumulate(Duration::from_millis(1100));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(cue.is_complete(), true);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal>()),
      Some(&Signal)
    );

    // Verify that accumulating more time doesn't change the state
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, false);
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
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(cue.is_complete(), false);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal>()),
      Some(&Signal)
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
    let mut cue = CueAccumulator::new(
      CueInternal::Loop(Duration::from_secs(1)),
      vec![Box::new(Signal)],
    );

    assert_eq!(cue.is_complete(), false);
    assert_eq!(cue.signal().is_none(), true);

    // Advance 1 second, should trigger first signal
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(cue.is_complete(), false);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal>()),
      Some(&Signal)
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
      CueInternal::Times(3, Duration::from_secs(1)),
      events![Signal, Signal2],
    );

    // Advance 1 second, should trigger first signal
    let result = cue.accumulate(Duration::from_millis(500));
    assert_eq!(result.completed_cycle, false);
    let result = cue.accumulate(Duration::from_millis(500));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal>()),
      Some(&Signal)
    );

    // Advance another second, should trigger second signal
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(cue.is_complete(), false);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal2>()),
      Some(&Signal2)
    );

    // Advance another second, should loop back around to the first signal
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(cue.is_complete(), true);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal>()),
      Some(&Signal)
    );
  }

  #[test]
  fn timeline_cue() {
    let mut cue = CueAccumulator::from_points(vec![
      (Duration::from_secs(1), Box::new(Signal)),
      (Duration::from_secs(2), Box::new(Signal2)),
      (Duration::from_secs(3), Box::new(Signal3)),
    ]);

    assert_eq!(cue.is_complete(), false);
    assert_eq!(cue.signal().is_none(), true);

    // Advance 1 second, should trigger first signal
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal>()),
      Some(&Signal)
    );

    // Advance another second, should trigger second signal
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal2>()),
      Some(&Signal2)
    );

    // Advance another second, should trigger third signal and complete
    let result = cue.accumulate(Duration::from_secs(1));
    assert_eq!(result.completed_cycle, true);
    assert_eq!(cue.is_complete(), true);
    assert_eq!(
      cue.signal().and_then(|s| s.downcast_ref::<Signal3>()),
      Some(&Signal3)
    );
  }
}
