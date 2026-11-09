use glam::{Affine2, Vec2};

/// An axis-aligned rectangle in scene units, such as a viewport or a shape's extent
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

  /// Normalizes a point to the unit box, `-0.5..=0.5` on each axis with the center at 0. A zero-length axis maps to
  /// `-0.5`.
  pub fn to_unit(&self, p: Vec2) -> Vec2 {
    let size = self.size();
    let offset = p - self.center();
    let axis = |offset: f32, size: f32| if size.abs() > f32::EPSILON { offset / size } else { -0.5 };
    Vec2::new(axis(offset.x, size.x), axis(offset.y, size.y))
  }

  /// [`Self::to_unit`] as an affine, except that a zero-length axis maps to 0
  pub fn to_unit_affine(&self) -> Affine2 {
    let size = self.size();
    let axis = |size: f32| if size.abs() > f32::EPSILON { 1.0 / size } else { 0.0 };
    Affine2::from_scale(Vec2::new(axis(size.x), axis(size.y))) * Affine2::from_translation(-self.center())
  }

  /// Converts a length to unit box units on each axis. A zero-length axis gives 0.
  pub fn to_unit_length(&self, length: f32) -> Vec2 {
    let size = self.size();
    let axis = |size: f32| if size.abs() > f32::EPSILON { length / size.abs() } else { 0.0 };
    Vec2::new(axis(size.x), axis(size.y))
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
  fn to_unit_length_scales_by_each_axis() {
    let rect = Bounds::new(Vec2::new(10.0, 0.0), Vec2::new(20.0, 4.0));
    assert_eq!(rect.to_unit_length(2.0), Vec2::new(0.2, 0.5));
    assert_eq!(Bounds::new(Vec2::ZERO, Vec2::new(0.0, 2.0)).to_unit_length(1.0), Vec2::new(0.0, 0.5));
  }

  #[test]
  fn to_unit_puts_a_flat_axis_at_its_start() {
    let rect = Bounds::new(Vec2::ZERO, Vec2::new(0.0, 2.0));
    assert_eq!(rect.to_unit(Vec2::new(0.0, 2.0)), Vec2::new(-0.5, 0.5));
  }
}
