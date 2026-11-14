use serde::{Deserialize, Serialize};

use crate::animation::Cycle;
use crate::scene::SceneError;

/// How far [`Timing::cycle_time`] keeps times from the edges of a cycle, as a fraction of it, so float rounding doesn't
/// push them into the neighboring cycle
const CYCLE_MARGIN: f32 = 1e-4;

/// How long an animated scene runs, and how many times. A scene without a timing is static: only its params can change
/// it.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Timing {
  /// Seconds per cycle
  pub duration: f32,
  /// How many times to play. Each set of keyframes can also loop on its own within a cycle.
  #[serde(default)]
  pub cycle: Cycle,
  /// What's shown once every cycle has played
  #[serde(default, skip_serializing_if = "EndBehavior::is_clear")]
  #[cfg_attr(feature = "ts", ts(as = "Option<EndBehavior>", optional))]
  pub end: EndBehavior,
}

impl Timing {
  pub fn new(duration: f32, cycle: Cycle) -> Self {
    Self { duration, cycle, end: EndBehavior::Clear }
  }

  pub fn end(mut self, end: EndBehavior) -> Self {
    self.end = end;
    self
  }

  /// Errors if the duration isn't a positive number
  pub fn check(&self) -> Result<(), SceneError> {
    match self.duration.is_finite() && self.duration > 0.0 {
      true => Ok(()),
      false => Err(SceneError::NoDuration),
    }
  }

  /// Whether more than one cycle plays
  pub(crate) fn repeats(&self) -> bool {
    !self.cycle.is_last(0)
  }

  pub(crate) fn local_time(&self, time: f32) -> f32 {
    self.cycle.local_time(time, self.duration)
  }

  pub fn cycle_index(&self, time: f32) -> u32 {
    self.cycle.index(time, self.duration)
  }

  pub(crate) fn is_last_cycle(&self, cycle: u32) -> bool {
    self.cycle.is_last(cycle)
  }

  pub fn is_complete(&self, time: f32) -> bool {
    self.cycle.is_complete(time, self.duration)
  }

  /// Seconds for every cycle to play, or `None` if it repeats forever
  pub fn length(&self) -> Option<f32> {
    match self.cycle {
      Cycle::Once => Some(self.duration),
      Cycle::Times(n) => Some(self.duration * n as f32),
      Cycle::Forever => None,
    }
  }

  /// Set to play 3 cycles, for previewing a typical cycle with neighbors on both sides (so random picks can blend across
  /// its edges). One that only plays once is unchanged. Use with [`Self::middle_cycle_time`].
  pub fn middle_cycle(self) -> Self {
    match self.repeats() {
      true => Self { cycle: Cycle::Times(3), ..self },
      false => self,
    }
  }

  /// Converts a time within a cycle to scene time in the middle cycle of [`Self::middle_cycle`]
  pub fn middle_cycle_time(&self, time: f32) -> f32 {
    self.cycle_time(1, time)
  }

  /// Converts a time within cycle number `cycle` to scene time. `time` can be the cycle's full duration and still land in
  /// that cycle. A cycle past the last is treated as the last. One that only plays once has only the one.
  pub fn cycle_time(&self, cycle: u32, time: f32) -> f32 {
    if !self.repeats() {
      return time;
    }
    let cycle = match self.cycle {
      Cycle::Times(times) => cycle.min(times - 1),
      _ => cycle,
    };
    self.duration * (cycle as f32 + (time / self.duration).clamp(CYCLE_MARGIN, 1.0 - CYCLE_MARGIN))
  }
}

/// What's shown once a scene has played every cycle
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EndBehavior {
  /// Nothing: it goes away
  #[default]
  Clear,
  /// Its last moment, until it's stopped
  Hold,
}

impl EndBehavior {
  fn is_clear(&self) -> bool {
    *self == Self::Clear
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn length_is_every_cycle() {
    assert_eq!(Timing::new(0.5, Cycle::Once).length(), Some(0.5));
    assert_eq!(Timing::new(0.5, Cycle::Times(4)).length(), Some(2.0));
    assert_eq!(Timing::new(0.5, Cycle::Forever).length(), None);
  }
}
