use glam::{Affine2, Vec2};
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Prop, Shape};

/// Moves, rotates, or scales a node
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transform {
  Translate(#[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))] Prop<Vec2>),
  /// Degrees clockwise (y points down), around the node's center as placed by the transforms before this one.
  ///
  /// A `Sequence` has no second axis, so there it scrolls the drawing along the line with the ends wrapped together:
  /// 360 degrees is one full lap.
  Rotate(Prop<f32>),
  Scale(#[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))] Prop<Vec2>),
}

impl Transform {
  /// The affine for this step, given where the transforms before it put the node's center
  pub(crate) fn affine(&self, center: Vec2, env: &Env) -> Affine2 {
    match self {
      Self::Translate(offset) => Affine2::from_translation(offset.resolve(env)),
      Self::Rotate(degrees) => match env.lap {
        Some(lap) => Affine2::from_translation(Vec2::new(lap * degrees.resolve(env) / 360.0, 0.0)),
        None => {
          let turn = Affine2::from_angle(degrees.resolve(env).to_radians());
          Affine2::from_translation(center) * turn * Affine2::from_translation(-center)
        }
      },
      Self::Scale(scale) => Affine2::from_scale(scale.resolve(env)),
    }
  }

  /// The path of a `Sweep` translate, if this is one
  pub(crate) fn path(&self) -> Option<&Shape> {
    match self {
      Self::Translate(Prop::Sweep(sweep)) => Some(&sweep.path),
      _ => None,
    }
  }

  pub(crate) fn is_animated(&self) -> bool {
    match self {
      Self::Translate(prop) | Self::Scale(prop) => prop.is_animated(),
      Self::Rotate(prop) => prop.is_animated(),
    }
  }
}
