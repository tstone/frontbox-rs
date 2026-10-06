use frontbox::prelude::ReferencePlane;
use std::sync::LazyLock;

/// What [`WebTracer::plane`](crate::WebTracer::plane) accepts: a plane `static`, or [`plane_path!`] of one to also let
/// the console copy positions as code (`planes::PLAYFIELD.to_absolute(Vec3::new(..))`).
pub trait IntoTracedPlane {
  /// The plane, and how the machine's code refers to it (e.g. `planes::PLAYFIELD`) if known
  fn into_traced_plane(self) -> (&'static ReferencePlane, Option<&'static str>);
}

impl<P: StaticPlane + ?Sized> IntoTracedPlane for &'static P {
  fn into_traced_plane(self) -> (&'static ReferencePlane, Option<&'static str>) {
    (self.reference_plane(), None)
  }
}

impl<P: StaticPlane + ?Sized> IntoTracedPlane for (&'static P, &'static str) {
  fn into_traced_plane(self) -> (&'static ReferencePlane, Option<&'static str>) {
    (self.0.reference_plane(), Some(self.1))
  }
}

/// Lets a plane be declared either as a plain `static` or as a `LazyLock` (needed when its rotation is computed, e.g.
/// with `Quat::from_axis_angle`, or it has an image).
pub trait StaticPlane {
  fn reference_plane(&'static self) -> &'static ReferencePlane;
}

impl StaticPlane for ReferencePlane {
  fn reference_plane(&'static self) -> &'static ReferencePlane {
    self
  }
}

impl StaticPlane for LazyLock<ReferencePlane> {
  fn reference_plane(&'static self) -> &'static ReferencePlane {
    self
  }
}

/// A plane `static` along with its path, so the console can copy positions as code.
///
/// ```rust,ignore
/// WebTracer::new().plane(plane_path!(planes::PLAYFIELD))
/// ```
#[macro_export]
macro_rules! plane_path {
  ($plane:path) => {
    (&$plane, stringify!($plane))
  };
}

#[cfg(test)]
mod tests {
  use super::IntoTracedPlane;
  use frontbox::prelude::{Quat, ReferencePlane, Vec2, Vec3};
  use std::sync::LazyLock;

  mod planes {
    use super::*;

    pub static PLAIN: ReferencePlane = ReferencePlane::new("Plain").extent(Vec2::new(20.0, 40.0)).build();

    pub static LAZY: LazyLock<ReferencePlane> = LazyLock::new(|| {
      ReferencePlane::new("Lazy")
        .extent(Vec2::new(10.0, 5.0))
        .rotation(Quat::from_axis_angle(Vec3::X, 90f32.to_radians()))
        .build()
    });
  }

  #[test]
  fn plane_path_records_the_path_for_plain_and_lazy_statics() {
    let (plain, code) = plane_path!(planes::PLAIN).into_traced_plane();
    assert_eq!(code, Some("planes::PLAIN"));
    assert_eq!(plain.extent, Vec2::new(20.0, 40.0));

    let (lazy, code) = plane_path!(planes::LAZY).into_traced_plane();
    assert_eq!(code, Some("planes::LAZY"));
    assert_eq!(lazy.extent, Vec2::new(10.0, 5.0));

    let (bare, code) = (&planes::LAZY).into_traced_plane();
    assert_eq!(code, None);
    assert_eq!(bare.name, "Lazy");
  }
}
