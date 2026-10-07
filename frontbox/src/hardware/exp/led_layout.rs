use std::f32::consts::TAU;

use crate::prelude::*;

/// Location generators for multi-LED definitions, e.g.
/// `LedDefinition::multi("arc").locations(LedLayout::strip(..).relative_to(&PLAYFIELD))`.
/// Layouts are built in a plane's own coordinates and then mapped onto it with `.relative_to`, so they follow the
/// plane's rotation. All angles are in radians, counter-clockwise from the plane's +x in its xy plane.
pub struct LedLayout;

impl LedLayout {
  /// A straight line starting at `origin` (first LED) with each subsequent LED `spacing` further along `rotation`,
  /// e.g. 0 = left to right along x-axis, 180deg/PI right to left along the x-axis, 90deg/PI/2 bottom to top along y-axis, etc.
  pub fn strip(count: u16, origin: Vec3, rotation: f32, spacing: f32) -> Vec<Vec3> {
    let step = heading(rotation) * spacing;
    (0..count).map(|i| origin + step * i as f32).collect()
  }

  /// Evenly spaced around a full circle, the first LED at `origin_rotation` and continuing in `direction`.
  pub fn ring(
    count: u16,
    center: Vec3,
    radius: f32,
    origin_rotation: f32,
    direction: LedLayoutDirection,
  ) -> Vec<Vec3> {
    Self::arc(count, center, radius, origin_rotation, TAU, direction)
  }

  /// Evenly spaced along part of a circle, the first LED at `rotation` and the last `sweep` further around in
  /// `direction`, e.g. a `sweep` of PI for a half circle. A full turn leaves a gap between the last and first LED
  /// rather than doubling them up.
  pub fn arc(
    count: u16,
    center: Vec3,
    radius: f32,
    rotation: f32,
    sweep: f32,
    direction: LedLayoutDirection,
  ) -> Vec<Vec3> {
    let sweep = sweep.abs();
    let full_turn = sweep >= TAU - 1e-4;
    let divisions = if full_turn {
      count
    } else {
      count.saturating_sub(1)
    }
    .max(1);
    let step = match direction {
      LedLayoutDirection::CounterClockwise => sweep,
      LedLayoutDirection::Clockwise => -sweep,
    } / divisions as f32;
    (0..count)
      .map(|i| center + heading(rotation + step * i as f32) * radius)
      .collect()
  }
}

/// Which way around a [`LedLayout::ring`] or [`LedLayout::arc`] the LEDs run
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedLayoutDirection {
  Clockwise,
  CounterClockwise,
}

fn heading(angle: f32) -> Vec3 {
  let (sin, cos) = angle.sin_cos();
  Vec3::new(cos, sin, 0.0)
}

#[cfg(test)]
mod tests {
  use std::f32::consts::{FRAC_PI_2, PI};

  use super::*;

  fn assert_near(actual: &[Vec3], expected: &[Vec3]) {
    assert_eq!(actual.len(), expected.len());
    for (a, e) in actual.iter().zip(expected) {
      assert!(a.distance(*e) < 1e-4, "{a} != {e}");
    }
  }

  #[test]
  fn layout_follows_plane_rotation() {
    // Stood upright, so the plane's +y points up the world's z
    let plane = ReferencePlane::new("Upright")
      .origin(Vec3::new(1.0, 0.0, 0.0))
      .rotation(Quat::from_rotation_x(FRAC_PI_2))
      .build();
    assert_near(
      &LedLayout::strip(3, Vec3::ZERO, FRAC_PI_2, 1.0).relative_to(&plane),
      &[Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 1.0), Vec3::new(1.0, 0.0, 2.0)],
    );
  }

  #[test]
  fn strip_steps_along_rotation() {
    assert_near(
      &LedLayout::strip(3, Vec3::new(1.0, 2.0, 3.0), FRAC_PI_2, 0.5),
      &[
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(1.0, 2.5, 3.0),
        Vec3::new(1.0, 3.0, 3.0),
      ],
    );
  }

  #[test]
  fn ring_spaces_evenly() {
    assert_near(
      &LedLayout::ring(
        4,
        Vec3::ZERO,
        2.0,
        0.0,
        LedLayoutDirection::CounterClockwise,
      ),
      &[
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
        Vec3::new(-2.0, 0.0, 0.0),
        Vec3::new(0.0, -2.0, 0.0),
      ],
    );
  }

  #[test]
  fn ring_runs_clockwise() {
    assert_near(
      &LedLayout::ring(4, Vec3::ZERO, 2.0, 0.0, LedLayoutDirection::Clockwise),
      &[
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, -2.0, 0.0),
        Vec3::new(-2.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
      ],
    );
  }

  #[test]
  fn arc_includes_both_ends() {
    assert_near(
      &LedLayout::arc(
        3,
        Vec3::ZERO,
        1.0,
        0.0,
        PI,
        LedLayoutDirection::CounterClockwise,
      ),
      &[
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
      ],
    );
  }

  #[test]
  fn arc_runs_clockwise() {
    assert_near(
      &LedLayout::arc(
        2,
        Vec3::ZERO,
        1.0,
        0.0,
        FRAC_PI_2,
        LedLayoutDirection::Clockwise,
      ),
      &[Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)],
    );
  }
}
