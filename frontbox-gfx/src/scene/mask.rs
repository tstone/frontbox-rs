use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::scene::{Bounds, Env, Fill, Sample, Shape, fill_outline, sample_fill};

/// Where a layer shows, as a fill in a shape of its own. The layer shows as much as the mask is opaque.
///
/// It's drawn like a layer but stays where it is in the scene, rather than following the layer's shape, so a mask that
/// moves reveals a layer that doesn't. It only changes the layer it's on, and what's drawn in it is never seen.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mask {
  pub fill: Fill,
  /// `None` covers the whole viewport
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub shape: Option<Shape>,
  /// Shows the layer outside the mask rather than inside it
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
  pub invert: bool,
}

impl Mask {
  pub fn new(fill: Fill) -> Self {
    Self { fill, shape: None, invert: false }
  }

  pub fn shape(mut self, shape: Shape) -> Self {
    self.shape = Some(shape);
    self
  }

  pub fn inverted(mut self) -> Self {
    self.invert = true;
    self
  }

  pub(crate) fn is_animated(&self) -> bool {
    self.fill.is_animated() || self.shape.as_ref().is_some_and(Shape::is_animated)
  }

  /// How much of the layer shows at a sample, from 0 to 1
  pub(crate) fn amount(&self, at: &Sample, viewport: Bounds, env: &Env) -> f32 {
    let opacity = sample_fill(&self.fill, self.shape.as_ref(), at, viewport, env)[3] as f32 / 255.0;
    if self.invert { 1.0 - opacity } else { opacity }
  }

  pub(crate) fn outline(&self, viewport: Bounds, env: &Env) -> Vec<Vec2> {
    fill_outline(self.shape.as_ref(), viewport, env)
  }
}
