use std::path::PathBuf;

use glam::{Quat, Vec2, Vec3};

use crate::prelude::ReferencePlane;

#[derive(Clone, Debug)]
pub struct ReferencePlaneBuilder {
  plane: ReferencePlane,
}

impl ReferencePlaneBuilder {
  pub const fn new(name: &'static str) -> Self {
    Self {
      plane: ReferencePlane {
        name,
        origin: Vec3::ZERO,
        extent: Vec2::ZERO,
        rotation: Quat::IDENTITY,
        parent: None,
        image: None,
      },
    }
  }

  /// Point in space (x,y,z) relative to the parent, or to the cabinet's bottom left corner without one
  pub const fn origin(mut self, origin: Vec3) -> Self {
    self.plane.origin = origin;
    self
  }

  /// Fixed size of the plane: width (x), height (y)
  pub const fn extent(mut self, extent: Vec2) -> Self {
    self.plane.extent = extent;
    self
  }

  pub const fn rotation(mut self, rotation: Quat) -> Self {
    self.plane.rotation = rotation;
    self
  }

  /// Plane this one's origin and rotation are relative to
  pub const fn parent(mut self, parent: &'static ReferencePlane) -> Self {
    self.plane.parent = Some(parent);
    self
  }

  /// An image stretched over the plane's extent (e.g. playfield art), so it should share the plane's aspect ratio.
  /// Relative paths are resolved from the process's working directory.
  pub fn image(mut self, path: impl Into<PathBuf>) -> Self {
    let path = path.into();
    if !path.exists() {
      log::warn!(
        "Image for reference plane \"{}\" not found at {}",
        self.plane.name,
        path.display()
      );
    }
    self.plane.image = Some(path);
    self
  }

  pub const fn build(self) -> ReferencePlane {
    // Moving `plane` out would drop the rest of `self`, which a const fn can't do (`image` has a destructor). There
    // is nothing else in `self`, so read `plane` out and forget the shell instead, letting planes be plain statics.
    let plane = unsafe { std::ptr::read(&self.plane) };
    std::mem::forget(self);
    plane
  }
}

impl From<ReferencePlaneBuilder> for ReferencePlane {
  fn from(builder: ReferencePlaneBuilder) -> Self {
    builder.build()
  }
}
