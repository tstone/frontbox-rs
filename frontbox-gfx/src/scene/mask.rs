use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::scene::{Bounds, Env, Fill, Sample, Shape, fill_outline, sample_fill};

/// Controls where a layer shows, using the alpha of the mask's own fill and region. The layer is visible in proportion
/// to the mask's opacity; the mask's colors are never drawn.
///
/// The mask is placed in the scene independently of the layer's region, so a moving mask can reveal a still layer.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mask {
  pub fill: Fill,
  /// `None` covers the whole viewport
  #[serde(default, alias = "shape", skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub region: Option<Shape>,
  /// Shows the layer outside the mask rather than inside it
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
  pub invert: bool,
}

impl Mask {
  pub fn new(fill: Fill) -> Self {
    Self { fill, region: None, invert: false }
  }

  pub fn region(mut self, region: Shape) -> Self {
    self.region = Some(region);
    self
  }

  pub fn inverted(mut self) -> Self {
    self.invert = true;
    self
  }

  pub(crate) fn is_animated(&self) -> bool {
    self.fill.is_animated() || self.region.as_ref().is_some_and(Shape::is_animated)
  }

  /// How visible the layer is at a sample, 0 to 1
  pub(crate) fn amount(&self, at: &Sample, viewport: Bounds, env: &Env) -> f32 {
    let opacity = sample_fill(&self.fill, self.region.as_ref(), at, viewport, env)[3] as f32 / 255.0;
    if self.invert { 1.0 - opacity } else { opacity }
  }

  pub(crate) fn outline(&self, viewport: Bounds, env: &Env) -> Vec<Vec2> {
    fill_outline(self.region.as_ref(), viewport, env)
  }
}
