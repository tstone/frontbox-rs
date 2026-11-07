use glam::{Affine2, Vec2};
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Prop, Shape};

/// Changes where a node is drawn
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transform {
  Translate(#[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))] Prop<Vec2>),
  /// Degrees, clockwise in a scene whose y runs down, around the middle of the drawing wherever the transforms before
  /// this have put it. A `Sequence` has no second axis to turn into, so there it runs the drawing around the line as if
  /// its ends met, toward the last point: 360 is one lap.
  Rotate(Prop<f32>),
  Scale(#[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))] Prop<Vec2>),
}

impl Transform {
  /// What this does to a point of the drawing, whose middle the transforms before this have put at `center`
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

  /// The path this moves the drawing along, if it does
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
