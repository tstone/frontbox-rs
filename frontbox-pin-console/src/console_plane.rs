use frontbox::prelude::ReferencePlane;
use std::path::PathBuf;

/// A surface of the machine (playfield, backbox, cabinet side, ...) for the console to draw, optionally with an
/// image such as playfield art laid over it.
///
/// ```rust,ignore
/// WebTracer::new()
///   .plane(ConsolePlane::new("Playfield", &PLAYFIELD).image("art/playfield.png"))
///   .plane(ConsolePlane::new("Cabinet left", &CABINET_LEFT))
/// ```
#[derive(Debug, Clone)]
pub struct ConsolePlane {
  /// Shown when hovering the plane in the console
  pub name: String,
  pub plane: &'static ReferencePlane,
  pub image: Option<PathBuf>,
}

impl ConsolePlane {
  pub fn new(name: impl Into<String>, plane: &'static ReferencePlane) -> Self {
    Self {
      name: name.into(),
      plane,
      image: None,
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
}
