use glam::{Mat4, Quat, Vec2, Vec3};

/// When specifying a location for hardware, it needs to be defined in 3d space (X,Y,Z)
/// However this can be complex and tricky to measure. In many cases it's easier to just define
/// a reference plane (e.g. the playfield surface, backbox surface, etc.) then define 2d coordinates
/// relative to that plane. They will be automatically mapped into 3d space.
///
/// ```rust
/// use frontbox::prelude::*;
/// 
/// let playfield: ReferencePlane = ReferencePlane {
///   // Origin is relative to the bottom left corner of the cabinet. Here the back left corner of the playfield
///   // is offset 1" from the left wall, 3.25" from the back wall, and 12" off the bottom fo the cabinet
///   origin: Vec3::new(1.0, 3.25, 12.0),
///   // playfield dimensions
///   extent: Vec2::new(20.25, 45.0),
///   // playfield is already relative to the bottom of the cabinet
///   rotation: Quat::IDENTITY,
///   parent: None,
/// };
/// ```
///
/// In some cases it's useful to express a reference plane relative to an existing plane. For example, an upper playfield
/// is already parallel to the playfield, and simply offset by some Z value. As an example, an upper playfield set in the
/// back right that is 6 inches wide and 4 inches above the playfield might be defined as...
///
/// ```rust
/// use frontbox::prelude::*;
/// # use std::sync::LazyLock;
/// # static PLAYFIELD: LazyLock<ReferencePlane> = LazyLock::new(|| { ReferencePlane {
///   origin: Vec3::new(1.0, 3.25, 12.0),
///   extent: Vec2::new(20.25, 45.0),
///   rotation: Quat::IDENTITY,
///   parent: None,
/// }});
/// 
/// let upper_playfield: ReferencePlane = ReferencePlane {
///   parent: Some(&PLAYFIELD),
///   origin: Vec3::new(14.25, 0.0, 4.0),
///   extent: Vec2::new(6.0, 6.0),
///   rotation: Quat::IDENTITY, // no rotation
/// };
/// ```
///
/// Rotation can also be specified for planes that are not parallel to the bottom of the cabinet. For example, the speaker
/// LEDs in the backbox might be defined on the plane that is the face of the backbox.
///
/// ```rust
/// use frontbox::prelude::*;
/// # use std::sync::LazyLock;
/// # static PLAYFIELD: LazyLock<ReferencePlane> = LazyLock::new(|| { ReferencePlane {
///   origin: Vec3::new(1.0, 3.25, 12.0),
///   extent: Vec2::new(20.25, 45.0),
///   rotation: Quat::IDENTITY,
///   parent: None,
/// }});
/// 
/// let backbox: ReferencePlane = ReferencePlane {
///   // specifying the top left of the backbox plane relative to the playfield. Making it relative to the playfield here
///   // so that this can be plane stitched later
///   origin: Vec3::new(0.0, 0.0, 32.0),
///   extent: Vec2::new(30.0, 32.0),
///   // Describe the backbox plane as perpendicular to the cabinet bottom
///   rotation: Quat::from_axis_angle(Vec3::X, 90f32.to_radians()),
///   parent: Some(&PLAYFIELD),
/// };
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReferencePlane {
  /// Point in space (x,y,z) relative to parent
  pub origin: Vec3,
  /// width (x), height (y)
  pub extent: Vec2,
  pub rotation: Quat,
  pub parent: Option<&'static ReferencePlane>,
}

impl ReferencePlane {
  pub fn new(
    origin: Vec3,
    extent: Vec2,
    rotation: Quat,
    parent: Option<&'static ReferencePlane>,
  ) -> Self {
    Self {
      origin,
      extent,
      rotation,
      parent,
    }
  }

  /// Map a given point relative to a plane to its absolute existence in the world
  pub fn to_absolute(&self, point: Vec3) -> Vec3 {
    let local = Mat4::from_rotation_translation(self.rotation, self.origin).transform_point3(point);
    match self.parent {
      Some(parent) => parent.to_absolute(local),
      None => local,
    }
  }

  /// Walk the parent chain to get this plane's origin/rotation in root space.
  pub fn world_transform(&self) -> (Vec3, Quat) {
    match self.parent {
      Some(parent) => {
        let (parent_origin, parent_rotation) = parent.world_transform();
        let world_origin = parent_origin + parent_rotation * self.origin;
        let world_rotation = parent_rotation * self.rotation;
        (world_origin, world_rotation)
      }
      None => (self.origin, self.rotation),
    }
  }

  /// Map a given absolute point to its coordinates on this plane
  pub fn to_relative(&self, point_world: Vec3) -> Vec2 {
    let (origin, rotation) = self.world_transform();
    let local = rotation.inverse() * (point_world - origin);
    Vec2::new(local.x, local.y)
  }
}

pub trait LocationRelativeTo {
  fn relative_to(&self, frame: &ReferencePlane) -> Vec3;
}

impl LocationRelativeTo for Vec2 {
  fn relative_to(&self, frame: &ReferencePlane) -> Vec3 {
    frame.to_absolute(Vec3::new(self.x, self.y, 0.0))
  }
}

impl LocationRelativeTo for Vec3 {
  fn relative_to(&self, frame: &ReferencePlane) -> Vec3 {
    frame.to_absolute(*self)
  }
}
