use glam::Vec2;

/// An axis-aligned area in scene units: a viewport, or a shape's extent
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
  pub min: Vec2,
  pub max: Vec2,
}

impl Bounds {
  pub fn new(min: Vec2, max: Vec2) -> Self {
    Self { min, max }
  }

  /// A rectangle from the origin to `size`
  pub fn sized(size: Vec2) -> Self {
    Self::new(Vec2::ZERO, size)
  }

  pub fn size(&self) -> Vec2 {
    self.max - self.min
  }

  pub fn contains(&self, p: Vec2) -> bool {
    p.cmpge(self.min).all() && p.cmple(self.max).all()
  }

  pub fn center(&self) -> Vec2 {
    (self.min + self.max) / 2.0
  }

  /// Maps a point in this rectangle to the unit box, `-0.5..=0.5` on each axis. An axis with no length puts
  /// everything at its start (`-0.5`).
  pub fn to_unit(&self, p: Vec2) -> Vec2 {
    let size = self.size();
    let offset = p - self.center();
    let axis = |offset: f32, size: f32| if size.abs() > f32::EPSILON { offset / size } else { -0.5 };
    Vec2::new(axis(offset.x, size.x), axis(offset.y, size.y))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn to_unit_maps_corners_to_the_unit_box() {
    let rect = Bounds::new(Vec2::new(10.0, 0.0), Vec2::new(20.0, 4.0));
    assert_eq!(rect.to_unit(Vec2::new(10.0, 0.0)), Vec2::new(-0.5, -0.5));
    assert_eq!(rect.to_unit(Vec2::new(20.0, 4.0)), Vec2::new(0.5, 0.5));
    assert_eq!(rect.to_unit(Vec2::new(15.0, 2.0)), Vec2::ZERO);
  }

  #[test]
  fn to_unit_puts_a_flat_axis_at_its_start() {
    let rect = Bounds::new(Vec2::ZERO, Vec2::new(0.0, 2.0));
    assert_eq!(rect.to_unit(Vec2::new(0.0, 2.0)), Vec2::new(-0.5, 0.5));
  }
}
