use std::collections::BTreeMap;

use glam::Vec2;
use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::animation::Lerp;

/// Parameter values by name
pub type Params = BTreeMap<String, Value>;

/// A parameter value the game supplies at runtime
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Value {
  Float(f32),
  Vec2(Vec2),
  Color(Color),
}

/// A color in a definition, written in files as `(r, g, b, a)`
///
/// `Rgba<u8>` belongs to the `image` crate, which has no serde support for it, so definitions hold this instead.
/// Sampling returns plain `Rgba<u8>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "(u8, u8, u8, u8)", into = "(u8, u8, u8, u8)")]
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

/// What props resolve against: the time, the game's parameters, and the scene's defaults
pub(crate) struct Env<'a> {
  pub time: f32,
  pub params: &'a Params,
  pub defaults: &'a Params,
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
    Env { time, params, defaults: params }
  }
}
