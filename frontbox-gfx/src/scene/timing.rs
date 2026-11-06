use serde::{Deserialize, Serialize};

use crate::animation::Cycle;

/// How long an animated scene runs. A scene without one is static: it can change through its parameters, but not on its
/// own.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Timing {
  /// Seconds in one cycle of the scene
  pub duration: f32,
  /// How many times to play it. Tracks have their own cycles within this.
  #[serde(default)]
  pub cycle: Cycle,
}

impl Timing {
  pub fn new(duration: f32, cycle: Cycle) -> Self {
    Self { duration, cycle }
  }

  pub(crate) fn local_time(&self, time: f32) -> f32 {
    self.cycle.local_time(time, self.duration)
  }

  pub(crate) fn cycle_index(&self, time: f32) -> u32 {
    self.cycle.index(time, self.duration)
  }

  pub(crate) fn is_last_cycle(&self, cycle: u32) -> bool {
    self.cycle.is_last(cycle)
  }

  pub(crate) fn is_complete(&self, time: f32) -> bool {
    self.cycle.is_complete(time, self.duration)
  }
}
