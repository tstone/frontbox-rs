use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::scene::Bounds;

/// The kind of surface a scene is drawn on. Its size comes from the device, through
/// [`FrameContext::size`](crate::scene::FrameContext::size).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Surface {
  /// A 1D line of points, such as an LED strip. Runs from x = 0 to x = 1 however many points there are.
  Sequence,
  /// A 2D rectangle from the origin to the device's size, in its units: pixels on a DMD, playfield units for LEDs
  Area,
}

impl Surface {
  /// The visible region in scene units. A `Sequence` is the same on every device.
  pub(crate) fn viewport(&self, size: Vec2) -> Bounds {
    match self {
      Self::Sequence => Bounds::new(Vec2::new(0.0, -0.5), Vec2::new(1.0, 0.5)),
      Self::Area => Bounds::sized(size),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_sequence_runs_from_its_first_point_to_its_last() {
    let viewport = Surface::Sequence.viewport(Vec2::new(100.0, 50.0));
    assert_eq!(viewport.to_unit(Vec2::ZERO).x, -0.5);
    assert_eq!(viewport.to_unit(Vec2::new(1.0, 0.0)).x, 0.5);
  }

  #[test]
  fn an_area_runs_from_the_origin() {
    let viewport = Surface::Area.viewport(Vec2::new(100.0, 50.0));
    assert_eq!(viewport.to_unit(Vec2::ZERO), Vec2::splat(-0.5));
    assert_eq!(viewport.to_unit(Vec2::new(100.0, 50.0)), Vec2::splat(0.5));
  }
}
