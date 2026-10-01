use frontbox::prelude::ReferencePlane;
use std::path::PathBuf;
use std::sync::LazyLock;

/// A surface of the machine (playfield, backbox, cabinet side, ...) for the console to draw, optionally with an
/// image such as playfield art laid over it.
///
/// Prefer building it with [`console_plane!`], which also records the plane's Rust path so the console can copy
/// positions as code (`planes::PLAYFIELD.to_absolute(Vec3::new(..))`):
///
/// ```rust,ignore
/// WebTracer::new()
///   .plane(console_plane!("Playfield", planes::PLAYFIELD).image("art/playfield.png"))
///   .plane(console_plane!("Cabinet left", planes::CABINET_LEFT))
/// ```
#[derive(Debug, Clone)]
pub struct ConsolePlane {
  /// Shown when hovering the plane in the console
  pub name: String,
  pub plane: &'static ReferencePlane,
  pub image: Option<PathBuf>,
  /// How the plane is referred to in the machine's code, e.g. `planes::PLAYFIELD`
  pub code: Option<String>,
}

impl ConsolePlane {
  pub fn new(name: impl Into<String>, plane: &'static ReferencePlane) -> Self {
    Self {
      name: name.into(),
      plane,
      image: None,
      code: None,
    }
  }

  /// An image stretched over the plane's extent, so it should share the plane's aspect ratio. Relative paths are
  /// resolved from the process's working directory. Around 2048-4096px on the long side works well; browsers
  /// scale down anything larger than the GPU allows.
  pub fn image(mut self, path: impl Into<PathBuf>) -> Self {
    let path = path.into();
    if !path.exists() {
      log::warn!(
        target: "frontbox_pin_console",
        "Image for console plane \"{}\" not found at {}",
        self.name,
        path.display()
      );
    }
    self.image = Some(path);
    self
  }

  /// How the plane is referred to in the machine's code, e.g. `planes::PLAYFIELD`. [`console_plane!`] fills this
  /// in from the path it's given.
  pub fn code(mut self, path: impl Into<String>) -> Self {
    self.code = Some(path.into());
    self
  }
}

/// Lets [`console_plane!`] take a plane declared either as a plain `static` or as a `LazyLock` (needed when its
/// rotation is computed, e.g. with `Quat::from_axis_angle`).
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

/// A [`ConsolePlane`] for a plane `static`, remembering its path so positions can be copied as code.
///
/// ```rust,ignore
/// console_plane!("Playfield", planes::PLAYFIELD)
/// ```
#[macro_export]
macro_rules! console_plane {
  ($name:expr, $plane:path) => {
    $crate::ConsolePlane::new($name, $crate::StaticPlane::reference_plane(&$plane)).code(stringify!($plane))
  };
}

#[cfg(test)]
mod tests {
  use frontbox::prelude::{Quat, ReferencePlane, Vec2, Vec3};
  use std::sync::LazyLock;

  mod planes {
    use super::*;

    pub static PLAIN: ReferencePlane = ReferencePlane {
      origin: Vec3::ZERO,
      extent: Vec2::new(20.0, 40.0),
      rotation: Quat::IDENTITY,
      parent: None,
    };

    pub static LAZY: LazyLock<ReferencePlane> = LazyLock::new(|| ReferencePlane {
      origin: Vec3::ZERO,
      extent: Vec2::new(10.0, 5.0),
      rotation: Quat::from_axis_angle(Vec3::X, 90f32.to_radians()),
      parent: None,
    });
  }

  #[test]
  fn console_plane_records_the_path_for_plain_and_lazy_statics() {
    let plain = console_plane!("Plain", planes::PLAIN);
    assert_eq!(plain.code.as_deref(), Some("planes::PLAIN"));
    assert_eq!(plain.plane.extent, Vec2::new(20.0, 40.0));

    let lazy = console_plane!("Lazy", planes::LAZY);
    assert_eq!(lazy.code.as_deref(), Some("planes::LAZY"));
    assert_eq!(lazy.plane.extent, Vec2::new(10.0, 5.0));
  }
}
