use std::collections::BTreeMap;

use glam::{Affine2, Vec2};
use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::animation::Lerp;
use crate::scene::{Assets, Bounds, NamedValue, NodeVisits, Timing, Values};

/// Variable values by name
pub type Variables = BTreeMap<String, Value>;

static NO_PARAMS: Variables = BTreeMap::new();

/// An empty set, for a scene with no variable defaults, or a frame that sets none
pub fn no_variables() -> &'static Variables {
  &NO_PARAMS
}

/// A variable value the game supplies at runtime
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
  Float(f32),
  Vec2(#[cfg_attr(feature = "ts", ts(as = "[f32; 2]"))] Vec2),
  Color(Color),
  /// A whole number, for counts and scores too large for a `Float` to hold exactly
  Int(i64),
  Text(String),
  /// Whether something is so, such as whether a layer is visible
  Bool(bool),
  /// One of the scene's named values, by name, written `Value("current")`
  #[serde(rename = "Value")]
  Named(String),
}

impl Value {
  /// Points a variable at one of the scene's named values
  pub fn named(name: impl Into<String>) -> Self {
    Self::Named(name.into())
  }
}

impl Lerp for Value {
  /// Between two values of the same type. Between different types, it's `other` the whole way.
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    match (self, other) {
      (Self::Float(from), Self::Float(to)) => Self::Float(from.interpolate(to, t)),
      (Self::Vec2(from), Self::Vec2(to)) => Self::Vec2(from.interpolate(to, t)),
      (Self::Color(from), Self::Color(to)) => Self::Color(from.interpolate(to, t)),
      (Self::Int(from), Self::Int(to)) => {
        Self::Int((*from as f64).interpolate(&(*to as f64), t).round() as i64)
      }
      _ => other.clone(),
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

impl From<i64> for Value {
  fn from(value: i64) -> Self {
    Self::Int(value)
  }
}

impl From<i32> for Value {
  fn from(value: i32) -> Self {
    Self::Int(value.into())
  }
}

impl From<u32> for Value {
  fn from(value: u32) -> Self {
    Self::Int(value.into())
  }
}

impl From<u64> for Value {
  /// Past `i64::MAX`, it's `i64::MAX`
  fn from(value: u64) -> Self {
    Self::Int(value.try_into().unwrap_or(i64::MAX))
  }
}

impl From<bool> for Value {
  fn from(value: bool) -> Self {
    Self::Bool(value)
  }
}

impl From<String> for Value {
  fn from(value: String) -> Self {
    Self::Text(value)
  }
}

impl From<&str> for Value {
  fn from(value: &str) -> Self {
    Self::Text(value.into())
  }
}

/// A color in a definition, written in files as `(r, g, b, a)`
///
/// `Rgba<u8>` belongs to the `image` crate, which has no serde support for it, so definitions hold this instead.
/// Sampling returns plain `Rgba<u8>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "(u8, u8, u8, u8)", into = "(u8, u8, u8, u8)")]
#[cfg_attr(
  feature = "ts",
  derive(ts_rs::TS),
  ts(type = "[number, number, number, number]")
)]
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

/// Everything a prop needs to resolve: time, variables, images and fonts, and the viewport
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
  pub variables: &'a Variables,
  pub defaults: &'a Variables,
  /// The scene's named values, which a variable can point at
  pub values: &'a Values,
  /// How many named values deep this is resolving, so values that point at each other stop
  pub depth: u8,
  pub assets: &'a Assets,
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

impl<'a> Env<'a> {
  pub(crate) fn variable(&self, name: &str) -> Option<&'a Value> {
    self.variables.get(name).or_else(|| self.defaults.get(name))
  }

  /// A named value, and the environment to resolve its prop in. `None` if there's none by that name, or if values have
  /// pointed at each other too many times over to be going anywhere.
  pub(crate) fn named(&self, name: &str) -> Option<(&'a NamedValue, Env<'a>)> {
    if self.depth >= MAX_VALUE_DEPTH {
      return None;
    }
    Some((
      self.values.get(name)?,
      Env {
        depth: self.depth + 1,
        ..*self
      },
    ))
  }
}

/// How many named values deep a variable can resolve
const MAX_VALUE_DEPTH: u8 = 8;

#[cfg(test)]
pub(crate) mod tests {
  use super::*;
  use crate::scene::no_values;

  pub(crate) fn env<'a>(time: f32, variables: &'a Variables) -> Env<'a> {
    Env {
      time,
      cycle: 0,
      timing: None,
      playback_seed: 0,
      variables,
      defaults: variables,
      values: no_values(),
      depth: 0,
      assets: Assets::none(),
      viewport: Bounds::sized(Vec2::new(100.0, 50.0)),
      lap: None,
      visits: None,
      fill_frame: Affine2::IDENTITY,
    }
  }
}
