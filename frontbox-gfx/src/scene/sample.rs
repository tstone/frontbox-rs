use glam::Vec2;

use crate::scene::{Images, Params, Viewport};

/// A point to get a color for: a position in scene units and the radius of the area it covers
///
/// `radius` is the size of what's being lit (half the pixel pitch on a DMD, the size of an insert on the playfield).
/// Shape edges use it for anti-aliasing: an edge passing through a sample covers it partially.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
  pub pos: Vec2,
  pub radius: f32,
}

impl Sample {
  pub fn new(pos: Vec2, radius: f32) -> Self {
    Self { pos, radius }
  }
}

/// Inputs shared by every sample in one frame
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
  /// Seconds since the scene started
  pub time: f32,
  /// Seed for random props. Players use a new one each time they restart a scene, so it doesn't repeat the same picks.
  pub take: u32,
  /// Values from the game, overriding the scene's param defaults
  pub params: &'a Params,
  /// Decoded images by name. A layer whose image is missing draws nothing; get these from
  /// [`Content::images_for`](crate::scene::Content::images_for), which reports missing images up front.
  pub images: &'a Images,
  /// Device size in its own units, which the scene's coordinates are in. Unused by a `Sequence`.
  pub size: Vec2,
  pub viewport: Viewport,
}
