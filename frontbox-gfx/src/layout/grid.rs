use glam::Vec2;

use crate::scene::Sample;

/// A `width` by `height` grid of points fit to an [`Area`](crate::scene::Viewport::Area) viewport, such as the pixels
/// of a DMD
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
  pub width: usize,
  pub height: usize,
  /// The size of the viewport
  pub size: Vec2,
}

impl Grid {
  pub fn new(width: usize, height: usize, size: Vec2) -> Self {
    Self {
      width,
      height,
      size,
    }
  }

  pub fn samples(&self) -> impl Iterator<Item = Sample> {
    let Self {
      width,
      height,
      size,
    } = *self;
    let cells = Vec2::new(width.max(1) as f32, height.max(1) as f32);
    let cell = (size / cells).max_element();
    let origin = (size - cells * cell) / 2.0;
    (0..height).flat_map(move |y| {
      (0..width).map(move |x| {
        let pos = origin + Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * cell;
        Sample::new(pos, cell / 2.0)
      })
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn points_sit_in_the_center_of_each_cell() {
    let samples: Vec<_> = Grid::new(3, 2, Vec2::new(3.0, 2.0)).samples().collect();
    assert_eq!(samples.len(), 6);
    assert_eq!(samples[0], Sample::new(Vec2::new(0.5, 0.5), 0.5));
    assert_eq!(samples[5], Sample::new(Vec2::new(2.5, 1.5), 0.5));
  }

  #[test]
  fn scales_with_the_grids_resolution() {
    let samples: Vec<_> = Grid::new(4, 4, Vec2::new(2.0, 2.0)).samples().collect();
    assert_eq!(samples[0], Sample::new(Vec2::new(0.25, 0.25), 0.25));
    assert_eq!(samples[15], Sample::new(Vec2::new(1.75, 1.75), 0.25));
  }

  #[test]
  fn a_wider_grid_keeps_square_cells_and_centers_the_viewport() {
    // 4 x 1 over a 2 x 2 viewport: each cell is 2 across, so the outer two land past the sides
    let samples: Vec<_> = Grid::new(4, 1, Vec2::new(2.0, 2.0)).samples().collect();
    let xs: Vec<_> = samples.iter().map(|s| s.pos.x).collect();
    assert_eq!(xs, vec![-2.0, 0.0, 2.0, 4.0]);
    assert!(samples.iter().all(|s| s.pos.y == 1.0 && s.radius == 1.0));
  }
}
