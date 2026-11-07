use glam::Vec2;
use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Fill, Bounds, Sample, Shape};

/// A fill drawn in a shape
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
  /// What people call it. It changes nothing about how it's drawn, and two layers can share one.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub name: Option<String>,
  /// Left out when the scene is drawn. It's still there to go back to, and still has to be something the scene can play.
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
  pub hidden: bool,
  pub fill: Fill,
  /// `None` covers the whole viewport
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub shape: Option<Shape>,
}

impl Layer {
  pub fn new(fill: Fill) -> Self {
    Self { name: None, hidden: false, fill, shape: None }
  }

  pub fn name(mut self, name: impl Into<String>) -> Self {
    self.name = Some(name.into());
    self
  }

  pub fn hidden(mut self) -> Self {
    self.hidden = true;
    self
  }

  pub fn shape(mut self, shape: Shape) -> Self {
    self.shape = Some(shape);
    self
  }

  /// How logs and errors should refer to the layer at `index` of a scene: by its name, or else its place from the bottom,
  /// counted from 1 as the console counts them
  pub fn label(&self, index: usize) -> String {
    match &self.name {
      Some(name) => format!("layer \"{name}\""),
      None => format!("layer {}", index + 1),
    }
  }

  pub(crate) fn is_animated(&self) -> bool {
    self.fill.is_animated() || self.shape.as_ref().is_some_and(Shape::is_animated)
  }

  pub(crate) fn sample(&self, at: &Sample, viewport: Bounds, env: &Env) -> Rgba<u8> {
    let (coverage, fill_pos, footprint) = match &self.shape {
      None => (1.0, viewport.to_unit(at.pos), viewport.to_unit_length(at.radius)),
      Some(shape) => match shape.cover(at, env) {
        Some((coverage, local, reach)) => {
          let bounds = shape.base.bounds(env);
          (coverage, bounds.to_unit(local), reach * bounds.to_unit_length(1.0))
        }
        None => return Rgba([0, 0, 0, 0]),
      },
    };

    let mut color = self.fill.color(fill_pos, footprint, env);
    color[3] = (color[3] as f32 * coverage).round() as u8;
    color
  }

  pub(crate) fn outline(&self, viewport: Bounds, env: &Env) -> Vec<Vec2> {
    match &self.shape {
      None => {
        let Bounds { min, max } = viewport;
        vec![min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)]
      }
      Some(shape) => shape.outline(env),
    }
  }
}
