use std::collections::BTreeMap;

use glam::{Affine2, Vec2};
use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::animation::Lerp;
use crate::scene::{Bounds, Images, NodeVisits, Timing};

/// Parameter values by name
pub type Params = BTreeMap<String, Value>;

static NO_PARAMS: Params = BTreeMap::new();

/// An empty set, for a scene with no param defaults, or a frame that sets none
pub fn no_params() -> &'static Params {
  &NO_PARAMS
}

/// A parameter value the game supplies at runtime
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Value {
  Float(f32),
  Vec2(#[cfg_attr(feature = "ts", ts(as = "[f32; 2]"))] Vec2),
  Color(Color),
}

impl Lerp for Value {
  /// Between two values of the same type. Between different types, it's `other` the whole way.
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    match (self, other) {
      (Self::Float(from), Self::Float(to)) => Self::Float(from.interpolate(to, t)),
      (Self::Vec2(from), Self::Vec2(to)) => Self::Vec2(from.interpolate(to, t)),
      (Self::Color(from), Self::Color(to)) => Self::Color(from.interpolate(to, t)),
      _ => *other,
    }
  }
}

impl From<f32> for Value {
  fn from(value: f32) -> Self {
    Self::Float(value)
  }
}

impl From<Vec2> for Value {
  fn from(value: Vec2) -> Self {
    Self::Vec2(value)
  }
}

impl From<Rgba<u8>> for Value {
  fn from(color: Rgba<u8>) -> Self {
    Self::Color(Color(color))
  }
}

/// A color in a definition, written in files as `(r, g, b, a)`
///
/// `Rgba<u8>` belongs to the `image` crate, which has no serde support for it, so definitions hold this instead.
/// Sampling returns plain `Rgba<u8>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "(u8, u8, u8, u8)", into = "(u8, u8, u8, u8)")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "[number, number, number, number]"))]
pub struct Color(pub Rgba<u8>);

impl Default for Color {
  /// Transparent
  fn default() -> Self {
    Self(Rgba([0, 0, 0, 0]))
  }
}

impl From<Rgba<u8>> for Color {
  fn from(color: Rgba<u8>) -> Self {
    Self(color)
  }
}

impl From<Color> for Rgba<u8> {
  fn from(color: Color) -> Self {
    color.0
  }
}

impl From<(u8, u8, u8, u8)> for Color {
  fn from((r, g, b, a): (u8, u8, u8, u8)) -> Self {
    Self(Rgba([r, g, b, a]))
  }
}

impl From<Color> for (u8, u8, u8, u8) {
  fn from(Color(Rgba([r, g, b, a])): Color) -> Self {
    (r, g, b, a)
  }
}

impl Lerp for Color {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    Self(self.0.interpolate(&other.0, t))
  }
}

/// Everything a prop needs to resolve: time, params, images, and the viewport
#[derive(Clone, Copy)]
pub(crate) struct Env<'a> {
  /// Seconds into the current scene cycle
  pub time: f32,
  /// Index of the current scene cycle, from 0
  pub cycle: u32,
  /// The scene's timing, so random picks can look into the neighboring cycles
  pub timing: Option<Timing>,
  /// See [`LayerFrame::take`](crate::scene::LayerFrame::playback_seed)
  pub playback_seed: u32,
  pub params: &'a Params,
  pub defaults: &'a Params,
  pub images: &'a Images,
  /// The visible region, in scene units
  pub viewport: Bounds,
  /// On a `Sequence`, the length of one wrap-around lap for `Rotate`, in the units of whatever is being drawn. `None`
  /// on an `Area`.
  pub lap: Option<f32>,
  /// At a graph node, its visits from the layer's traversals, for [`Prop::Visit`](crate::scene::Prop::Visit)
  pub visits: Option<NodeVisits<'a>>,
  /// Scene units to the unit box of the fill being drawn, for `Transform::Fit`. Identity outside a fill.
  pub fill_frame: Affine2,
}

impl Env<'_> {
  pub(crate) fn param(&self, name: &str) -> Option<&Value> {
    self.params.get(name).or_else(|| self.defaults.get(name))
  }
}

#[cfg(test)]
pub(crate) mod tests {
  use super::*;

  pub(crate) fn env<'a>(time: f32, params: &'a Params) -> Env<'a> {
    Env { time, cycle: 0, timing: None, playback_seed: 0, params, defaults: params, images: Images::none(), viewport: Bounds::sized(Vec2::new(100.0, 50.0)), lap: None, visits: None, fill_frame: Affine2::IDENTITY }
  }
}
