use glam::Vec2;

use crate::scene::Sample;

/// `count` points evenly spaced along a [`Sequence`](crate::scene::Viewport::Sequence) viewport, such as a strip of
/// LEDs
///
/// The first point lands at 0 and the last at 1, whatever the count, so a gradient puts its end colors exactly on the
/// end LEDs, and a shape covers the same part of any strip. A lone point sits at the start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line {
  pub count: usize,
}

impl Line {
  pub fn new(count: usize) -> Self {
    Self { count }
  }

  pub fn samples(&self) -> impl Iterator<Item = Sample> {
    let steps = self.count.saturating_sub(1) as f32;
    let radius = if steps > 0.0 { 0.5 / steps } else { 0.5 };
    (0..self.count).map(move |i| {
      let x = if steps > 0.0 { i as f32 / steps } else { 0.0 };
      Sample::new(Vec2::new(x, 0.0), radius)
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn positions(line: Line) -> Vec<f32> {
    line.samples().map(|s| s.pos.x).collect()
  }

  #[test]
  fn points_spread_from_end_to_end() {
    assert_eq!(positions(Line::new(5)), vec![0.0, 0.25, 0.5, 0.75, 1.0]);
    assert_eq!(positions(Line::new(3)), vec![0.0, 0.5, 1.0]);
  }

  #[test]
  fn the_radius_is_half_the_spacing() {
    assert_eq!(Line::new(5).samples().next().unwrap().radius, 0.125);
  }

  #[test]
  fn the_last_point_lands_on_the_end_of_any_line() {
    for count in 2..200 {
      assert_eq!(positions(Line::new(count)).last(), Some(&1.0), "{count} points");
    }
  }

  #[test]
  fn a_lone_point_sits_at_the_start() {
    assert_eq!(positions(Line::new(1)), vec![0.0]);
  }
}
