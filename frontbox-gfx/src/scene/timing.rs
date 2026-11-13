use serde::{Deserialize, Serialize};

use crate::animation::Cycle;

/// How long an animated scene runs, and how many times. A scene without a timing is static: only its params can change
/// it.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Timing {
  /// Seconds per cycle
  pub duration: f32,
  /// How many times to play. Each track can also loop on its own within a cycle.
  #[serde(default)]
  pub cycle: Cycle,
}

impl Timing {
  pub fn new(duration: f32, cycle: Cycle) -> Self {
    Self { duration, cycle }
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
