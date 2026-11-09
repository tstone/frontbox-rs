use std::collections::BTreeMap;

use glam::Vec2;
use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::animation::Lerp;
use crate::scene::{Bounds, Images, Timing};

/// Parameter values by name
pub type Params = BTreeMap<String, Value>;

/// A parameter value the game supplies at runtime
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Value {
  Float(f32),
  Vec2(#[cfg_attr(feature = "ts", ts(as = "[f32; 2]"))] Vec2),
  Color(Color),
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
pub(crate) struct Env<'a> {
  /// Seconds into the current scene cycle
  pub time: f32,
  /// Index of the current scene cycle, from 0
  pub cycle: u32,
  /// The scene's timing, so random picks can look into the neighboring cycles
  pub timing: Option<Timing>,
  /// See [`Frame::take`](crate::scene::Frame::take)
  pub take: u32,
  pub params: &'a Params,
  pub defaults: &'a Params,
  pub images: &'a Images,
  /// The visible region, in scene units
  pub viewport: Bounds,
  /// On a `Sequence`, the length of one wrap-around lap for `Rotate`, in the units of whatever is being drawn. `None`
  /// on an `Area`.
  pub lap: Option<f32>,
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
    Env { time, cycle: 0, timing: None, take: 0, params, defaults: params, images: Images::none(), viewport: Bounds::sized(Vec2::new(100.0, 50.0)), lap: None }
  }
}
