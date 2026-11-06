use glam::Vec2;

use crate::scene::Params;

/// A point a device wants a color for: a position in scene units and the radius of the area it covers
///
/// `radius` is the footprint of what's being lit (half the pixel pitch on a DMD, the size of an insert on the
/// playfield). Shape edges use it for coverage, so an edge passing through a sample lights it partially.
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

/// What's the same for every sample in one frame
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
  /// Seconds since the scene started
  pub time: f32,
  /// Which set of random picks to show. Anything that plays a scene takes a new one each time it starts it over.
  pub take: u32,
  /// Parameters supplied by the game, which take the place of the scene's defaults
  pub params: &'a Params,
  /// The size of the device, in its own units. A scene's numbers are in these units. A `Sequence` doesn't use this.
  pub size: Vec2,
}
