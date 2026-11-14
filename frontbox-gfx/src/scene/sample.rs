use glam::Vec2;

use crate::scene::{Assets, Params, Surface, Timing, Visits};

/// A point to get a color for: a position in scene units and the radius of the area it covers
///
/// `radius` is the size of what's being lit (half the pixel pitch on a DMD, the size of an insert on the playfield).
/// Shape edges use it for anti-aliasing: an edge passing through a sample covers it partially.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
  pub pos: Vec2,
  pub radius: f32,
  /// Its index in the [`Graph`](crate::scene::Graph) built from the samples, for layers with a graph. Those layers leave
  /// out samples without one.
  pub node: Option<usize>,
}

impl Sample {
  pub fn new(pos: Vec2, radius: f32) -> Self {
    Self { pos, radius, node: None }
  }

  pub fn at_node(mut self, node: usize) -> Self {
    self.node = Some(node);
    self
  }
}

/// Inputs shared by every sample of a layer in one frame
#[derive(Debug, Clone, Copy)]
pub struct LayerFrame<'a> {
  /// Seconds since the scene started
  pub time: f32,
  /// The scene's timing. `None` for a static scene, whose keyframes stay at time 0.
  pub timing: Option<Timing>,
  /// Seed for random props. Players use a new one each time they restart a scene, so it doesn't repeat the same picks.
  pub playback_seed: u32,
  /// Values from the game, overriding the scene's param defaults
  pub params: &'a Params,
  /// The scene's param defaults
  pub defaults: &'a Params,
  /// Decoded images and fonts by name. A layer whose image or font is missing draws nothing; get these from
  /// [`Content::assets_for`](crate::scene::Content::assets_for), which reports missing ones up front.
  pub assets: &'a Assets,
  /// Size of what the layer is drawn on, in its own units, which the layer's coordinates are in. Unused by a `Sequence`.
  pub size: Vec2,
  pub surface: Surface,
  /// The layer's [`Layer::visits`](crate::scene::Layer::visits) this frame, for a layer with a graph. Without it, its
  /// visit responses stay at rest.
  pub visits: Option<&'a Visits>,
}
