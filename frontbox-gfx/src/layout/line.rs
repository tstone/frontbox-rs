use glam::Vec2;

use crate::scene::{Bounds, Sample};

/// `count` points evenly spaced along a line, such as a strip of LEDs
///
/// Point `i` sits at `(i, 0)` with a radius of 0.5, and the stage runs from the first point's center to the last's, so
/// a gradient across the stage puts its end colors exactly on the end points. Positions on the line are indexes: a
/// shape centered at `(2, 0)` is centered on the third point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line {
  pub count: usize,
}

impl Line {
  pub fn new(count: usize) -> Self {
    Self { count }
  }

  pub fn stage(&self) -> Bounds {
    let end = self.count.saturating_sub(1) as f32;
    Bounds::new(Vec2::new(0.0, -0.5), Vec2::new(end, 0.5))
  }

  pub fn samples(&self) -> impl Iterator<Item = Sample> {
    (0..self.count).map(|i| Sample::new(Vec2::new(i as f32, 0.0), 0.5))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn line_spans_its_led_centers() {
    let line = Line::new(5);
    let samples: Vec<_> = line.samples().collect();
    assert_eq!(samples.len(), 5);
    assert_eq!(samples[0].pos, Vec2::new(0.0, 0.0));
    assert_eq!(samples[4].pos, Vec2::new(4.0, 0.0));
    assert_eq!(line.stage().to_unit(samples[0].pos).x, -0.5);
    assert_eq!(line.stage().to_unit(samples[4].pos).x, 0.5);
  }
}
