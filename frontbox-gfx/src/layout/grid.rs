use glam::Vec2;

use crate::scene::{Bounds, Sample};

/// A `width` by `height` grid of sample points fit to an [`Area`](crate::scene::Surface::Area) viewport, such as the
/// pixels of a DMD
///
/// Cells are always square. If the grid's aspect ratio differs from `size`, it fills `size` along one axis and runs past
/// it, centered, along the other.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
  pub width: usize,
  pub height: usize,
  /// Size of the viewport, in scene units
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
    let Self { width, height, .. } = *self;
    let Fit { origin, cell } = self.fit();
    (0..height).flat_map(move |y| {
      (0..width).map(move |x| {
        let pos = origin + Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * cell;
        Sample::new(pos, cell / 2.0)
      })
    })
  }

  /// The area the grid spans, in scene units. This can extend past `size` on one axis (see [`Grid`]).
  pub fn covers(&self) -> Bounds {
    let Fit { origin, cell } = self.fit();
    Bounds::new(origin, origin + self.cells() * cell)
  }

  fn cells(&self) -> Vec2 {
    Vec2::new(self.width.max(1) as f32, self.height.max(1) as f32)
  }

  fn fit(&self) -> Fit {
    let cells = self.cells();
    let cell = (self.size / cells).max_element();
    Fit { origin: (self.size - cells * cell) / 2.0, cell }
  }
}

/// Where the grid sits over its viewport, in scene units
struct Fit {
  /// Top left corner
  origin: Vec2,
  /// Side length of a cell
  cell: f32,
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

  #[test]
  fn a_grid_of_the_same_shape_covers_exactly_the_size() {
    let covers = Grid::new(4, 2, Vec2::new(100.0, 50.0)).covers();
    assert_eq!(covers, Bounds::sized(Vec2::new(100.0, 50.0)));
  }

  #[test]
  fn a_wider_grid_covers_past_the_sides() {
    let covers = Grid::new(4, 1, Vec2::new(2.0, 2.0)).covers();
    assert_eq!(covers, Bounds::new(Vec2::new(-3.0, 0.0), Vec2::new(5.0, 2.0)));
  }

  #[test]
  fn a_taller_grid_covers_past_the_top_and_bottom() {
    let covers = Grid::new(1, 4, Vec2::new(2.0, 2.0)).covers();
    assert_eq!(covers, Bounds::new(Vec2::new(0.0, -3.0), Vec2::new(2.0, 5.0)));
  }

  #[test]
  fn what_a_grid_covers_lines_up_with_its_samples() {
    let grid = Grid::new(128, 32, Vec2::new(12.0, 22.0));
    let covers = grid.covers();
    let cell = covers.size() / Vec2::new(128.0, 32.0);
    for (i, sample) in grid.samples().enumerate() {
      let at = covers.min + Vec2::new((i % 128) as f32 + 0.5, (i / 128) as f32 + 0.5) * cell;
      assert!(at.abs_diff_eq(sample.pos, 1e-3), "{at} vs {}", sample.pos);
    }
  }

  #[test]
  fn a_grid_over_no_size_covers_nothing() {
    let covers = Grid::new(4, 2, Vec2::ZERO).covers();
    assert_eq!(covers.size(), Vec2::ZERO);
  }
}
