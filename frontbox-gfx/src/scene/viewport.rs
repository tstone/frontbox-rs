use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::scene::Bounds;

/// What the scene is seen within
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Viewport {
  /// Points in order, such as a list of LEDs (1D space)
  Sequence,
  /// A rectangle from the origin (2D space)
  Area(#[cfg_attr(feature = "ts", ts(as = "[f32; 2]"))] Vec2),
}

impl Viewport {
  pub(crate) fn bounds(&self) -> Bounds {
    match self {
      Self::Sequence => Bounds::new(Vec2::new(0.0, -0.5), Vec2::new(1.0, 0.5)),
      Self::Area(size) => Bounds::sized(*size),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_sequence_runs_from_its_first_point_to_its_last() {
    let bounds = Viewport::Sequence.bounds();
    assert_eq!(bounds.to_unit(Vec2::ZERO).x, -0.5);
    assert_eq!(bounds.to_unit(Vec2::new(1.0, 0.0)).x, 0.5);
  }

  #[test]
  fn an_area_runs_from_the_origin() {
    let bounds = Viewport::Area(Vec2::new(100.0, 50.0)).bounds();
    assert_eq!(bounds.to_unit(Vec2::ZERO), Vec2::splat(-0.5));
    assert_eq!(bounds.to_unit(Vec2::new(100.0, 50.0)), Vec2::splat(0.5));
  }
}
