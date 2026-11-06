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
  /// Parameters supplied by the game, which take the place of the scene's defaults
  pub params: &'a Params,
}
