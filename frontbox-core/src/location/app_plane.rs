use crate::location::ReferencePlane;
use std::sync::LazyLock;

/// A surface of the machine. Scene files find planes by name among those added to the app, and tracers like the console
/// are given them when they start. `code` is only set by [`app_plane!`], and lets positions on the plane be
/// copied as code.
#[derive(Debug, Clone, Copy)]
pub struct AppPlane {
  pub plane: &'static ReferencePlane,
  /// How the plane is referred to in code, e.g. `planes::PLAYFIELD`
  pub code: Option<&'static str>,
}

impl From<&'static ReferencePlane> for AppPlane {
  fn from(plane: &'static ReferencePlane) -> Self {
    Self { plane, code: None }
  }
}

impl From<&'static LazyLock<ReferencePlane>> for AppPlane {
  fn from(plane: &'static LazyLock<ReferencePlane>) -> Self {
    Self { plane, code: None }
  }
}

/// Lets [`app_plane!`] take a plane declared either as a plain `static` or as a `LazyLock`
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

/// An [`AppPlane`] that records the plane's path. Taking the path itself makes a typo or rename a compile error
/// rather than wrong copied code.
///
/// ```rust,ignore
/// app.plane(app_plane!(planes::PLAYFIELD));
/// app.plane(app_plane!(planes::CABINET_FRONT));
/// ```
#[macro_export]
macro_rules! app_plane {
  ($plane:path) => {
    $crate::prelude::AppPlane {
      plane: $crate::prelude::StaticPlane::reference_plane(&$plane),
      code: Some(stringify!($plane)),
    }
  };
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::{Quat, Vec2, Vec3};

  mod planes {
    use super::*;

    pub static PLAIN: ReferencePlane = ReferencePlane::new("Plain")
      .extent(Vec2::new(20.0, 40.0))
      .build();

    pub static LAZY: LazyLock<ReferencePlane> = LazyLock::new(|| {
      ReferencePlane::new("Lazy")
        .extent(Vec2::new(10.0, 5.0))
        .rotation(Quat::from_axis_angle(Vec3::X, 90f32.to_radians()))
        .build()
    });
  }

  #[test]
  fn app_plane_records_the_path_for_plain_and_lazy_statics() {
    let plain = crate::app_plane!(planes::PLAIN);
    assert_eq!(plain.code, Some("planes::PLAIN"));
    assert_eq!(plain.plane.name, "Plain");

    let lazy = crate::app_plane!(planes::LAZY);
    assert_eq!(lazy.code, Some("planes::LAZY"));
    assert_eq!(lazy.plane.name, "Lazy");
  }

  #[test]
  fn planes_without_the_macro_have_no_path() {
    assert_eq!(AppPlane::from(&planes::PLAIN).code, None);
    assert_eq!(AppPlane::from(&planes::LAZY).code, None);
  }
}
