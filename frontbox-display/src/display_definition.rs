use crate::{DisplayDevice, DisplayMirror, DisplaySize};

/// A display to add to a [`DisplaySystem`](crate::DisplaySystem): its device, the name scene files call it by, and
/// anything else that shows its frames
///
/// ```rust,ignore
/// app.system(DisplaySystem::new().display(
///   DisplayDefinition::new("Dmd", Pin2Dmd::new(128, 32, PanelType::Rgb)).mirror(console.display_view()),
/// ));
/// ```
pub struct DisplayDefinition {
  pub(crate) name: String,
  pub(crate) size: DisplaySize,
  pub(crate) device: Box<dyn DisplayDevice>,
  pub(crate) hard_edges: bool,
  pub(crate) mirrors: Vec<Box<dyn DisplayMirror>>,
}

impl DisplayDefinition {
  /// `name` is what a scene's `Display` layers name it by. Its size is the device's.
  pub fn new(name: impl Into<String>, device: impl DisplayDevice) -> Self {
    Self { name: name.into(), size: device.size(), device: Box::new(device), hard_edges: true, mirrors: Vec::new() }
  }

  /// Shape edges partly cover the pixels they pass through, rather than each pixel being in or out. That's smoother on a
  /// display with small pixels, and blurry on a DMD.
  pub fn soft_edges(mut self) -> Self {
    self.hard_edges = false;
    self
  }

  /// Also shows its frames on `mirror`, even on a virtual platform, where the device isn't opened
  pub fn mirror(mut self, mirror: impl DisplayMirror) -> Self {
    self.mirrors.push(Box::new(mirror));
    self
  }
}
