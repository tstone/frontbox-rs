use glam::Vec2;

use crate::scene::{Bounds, Sample};

/// A `width` by `height` grid of points covering a stage, such as the pixels of a DMD
///
/// Points sit at the center of each cell, row by row from the top left, with a radius of half the cell's longer side.
/// The stage defaults to one unit per cell, so positions on it are pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
  pub width: usize,
  pub height: usize,
  pub stage: Bounds,
}

impl Grid {
  pub fn new(width: usize, height: usize) -> Self {
    let stage = Bounds::sized(Vec2::new(width as f32, height as f32));
    Self { width, height, stage }
  }

  /// Spreads the grid over a different stage, such as a sized scene's
  pub fn over(mut self, stage: Bounds) -> Self {
    self.stage = stage;
    self
  }

  pub fn stage(&self) -> Bounds {
    self.stage
  }

  pub fn samples(&self) -> impl Iterator<Item = Sample> {
    let Self { width, height, stage } = *self;
    let cell = stage.size() / Vec2::new(width.max(1) as f32, height.max(1) as f32);
    let radius = cell.max_element() / 2.0;
    (0..height).flat_map(move |y| {
      (0..width).map(move |x| {
        let pos = stage.min + cell * Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
        Sample::new(pos, radius)
      })
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn points_sit_in_the_center_of_each_pixel() {
    let samples: Vec<_> = Grid::new(3, 2).samples().collect();
    assert_eq!(samples.len(), 6);
    assert_eq!(samples[0], Sample::new(Vec2::new(0.5, 0.5), 0.5));
    assert_eq!(samples[5], Sample::new(Vec2::new(2.5, 1.5), 0.5));
  }

  #[test]
  fn spreads_over_a_sized_stage() {
    let grid = Grid::new(2, 1).over(Bounds::sized(Vec2::new(100.0, 50.0)));
    let samples: Vec<_> = grid.samples().collect();
    assert_eq!(
      samples,
      vec![
        Sample::new(Vec2::new(25.0, 25.0), 25.0),
        Sample::new(Vec2::new(75.0, 25.0), 25.0),
      ]
    );
  }
}
