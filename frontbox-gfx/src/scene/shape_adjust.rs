use glam::Vec2;
use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::scene::{Color, Env, Inputs, Prop};

/// Changes to a shape's edge, applied in order after its transforms
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeAdjust {
  /// A band `width` across along the edge, in scene units, so a `Scale` before it doesn't thicken it. With no `color`,
  /// the shape becomes the band, drawn with the layer's fill, and the inside is left out. With one, the band is drawn
  /// in that color over the shape. Each copy a `Mirror` or `Revolve` makes gets its own band.
  Stroke {
    width: Prop<f32>,
    #[serde(default, skip_serializing_if = "StrokeAlign::is_centered")]
    #[cfg_attr(feature = "ts", ts(as = "Option<StrokeAlign>", optional))]
    align: StrokeAlign,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    color: Option<Prop<Color>>,
  },
  /// Softens the edge across a band `width` wide, in scene units, centered on it, as a blur of the shape would: a
  /// sample on the edge is half covered, and coverage eases to nothing `width / 2` outside it. It softens each colored
  /// stroke's band too. Feathers add up, wherever they are in the list. With a solid fill and an outside stroke before
  /// it, it's a glow; moved with a `Translate`, a drop shadow.
  Feather { width: Prop<f32> },
  /// The shape again, under it, in `color`: moved by `offset` and grown by `spread`, both in scene units, then softened
  /// across `blur` as a [`Feather`](Self::Feather) softens. With no offset it's a glow around the shape. It follows the
  /// edge strokes with no color leave, wherever they are in the list, and isn't feathered with the shape. Each shadow
  /// is drawn over the ones before it.
  Shadow {
    #[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))]
    offset: Prop<Vec2>,
    blur: Prop<f32>,
    spread: Prop<f32>,
    color: Prop<Color>,
  },
}

/// Which side of the edge a stroke's band is on
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrokeAlign {
  Inside,
  #[default]
  Centered,
  Outside,
}

impl ShapeAdjust {
  pub(crate) fn prepare(&self, env: &Env) -> PreparedAdjust {
    match self {
      Self::Stroke {
        width,
        align,
        color,
      } => PreparedAdjust::Stroke(PreparedStroke {
        width: width.resolve(env),
        align: *align,
        color: color.as_ref().map(|color| color.resolve(env).0),
      }),
      Self::Feather { width } => PreparedAdjust::Feather(width.resolve(env).max(0.0)),
      Self::Shadow {
        offset,
        blur,
        spread,
        color,
      } => PreparedAdjust::Shadow(PreparedShadow {
        offset: offset.resolve(env),
        blur: blur.resolve(env).max(0.0),
        spread: spread.resolve(env),
        color: color.resolve(env).0,
      }),
    }
  }

  pub(crate) fn is_animated(&self) -> bool {
    match self {
      Self::Stroke { width, color, .. } => {
        width.is_animated() || color.as_ref().is_some_and(Prop::is_animated)
      }
      Self::Feather { width } => width.is_animated(),
      Self::Shadow {
        offset,
        blur,
        spread,
        color,
      } => offset.is_animated() || blur.is_animated() || spread.is_animated() || color.is_animated(),
    }
  }

  pub(crate) fn read_inputs(&self, inputs: &mut Inputs) {
    match self {
      Self::Stroke { width, color, .. } => {
        width.read_inputs(inputs);
        color.iter().for_each(|color| color.read_inputs(inputs));
      }
      Self::Feather { width } => width.read_inputs(inputs),
      Self::Shadow {
        offset,
        blur,
        spread,
        color,
      } => {
        offset.read_inputs(inputs);
        blur.read_inputs(inputs);
        spread.read_inputs(inputs);
        color.read_inputs(inputs);
      }
    }
  }
}

impl StrokeAlign {
  fn is_centered(&self) -> bool {
    *self == Self::Centered
  }
}

/// A [`ShapeAdjust`] with its props resolved
pub(crate) enum PreparedAdjust {
  Stroke(PreparedStroke),
  /// How wide, never below 0
  Feather(f32),
  Shadow(PreparedShadow),
}

/// A [`ShapeAdjust::Shadow`] with its props resolved
#[derive(Debug, Clone, Copy)]
pub(crate) struct PreparedShadow {
  pub offset: Vec2,
  /// Never below 0
  pub blur: f32,
  pub spread: f32,
  pub color: Rgba<u8>,
}

impl PreparedShadow {
  /// How far past the shape's edge it reaches, before its offset
  pub(crate) fn reach(&self) -> f32 {
    (self.spread + self.blur / 2.0).max(0.0)
  }
}

/// A [`ShapeAdjust::Stroke`] with its props resolved
#[derive(Debug, Clone, Copy)]
pub(crate) struct PreparedStroke {
  width: f32,
  align: StrokeAlign,
  /// `None` when the shape becomes the band
  pub color: Option<Rgba<u8>>,
}

impl PreparedStroke {
  /// Signed distance to the band (negative inside it), given the signed distance to the edge it runs along
  pub(crate) fn band(&self, distance: f32) -> f32 {
    if self.width <= 0.0 {
      return f32::INFINITY;
    }
    let half = self.width / 2.0;
    let middle = match self.align {
      StrokeAlign::Inside => -half,
      StrokeAlign::Centered => 0.0,
      StrokeAlign::Outside => half,
    };
    (distance - middle).abs() - half
  }

  /// How far past the edge the band reaches
  pub(crate) fn reach(&self) -> f32 {
    match self.align {
      StrokeAlign::Inside => 0.0,
      StrokeAlign::Centered => self.width.max(0.0) / 2.0,
      StrokeAlign::Outside => self.width.max(0.0),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn stroke(width: f32, align: StrokeAlign) -> PreparedStroke {
    PreparedStroke {
      width,
      align,
      color: None,
    }
  }

  #[test]
  fn a_band_is_on_the_side_of_the_edge_it_is_aligned_to() {
    for (align, inside, outside) in [
      (StrokeAlign::Inside, -2.0, 0.0),
      (StrokeAlign::Centered, -1.0, 1.0),
      (StrokeAlign::Outside, 0.0, 2.0),
    ] {
      let band = stroke(2.0, align);
      assert_eq!(band.band(inside), 0.0, "{align:?}");
      assert_eq!(band.band(outside), 0.0, "{align:?}");
      assert!(band.band((inside + outside) / 2.0) < 0.0, "{align:?}");
      assert_eq!(band.band(outside + 1.0), 1.0, "{align:?}");
      assert_eq!(band.reach(), outside, "{align:?}");
    }
  }

  #[test]
  fn a_band_with_no_width_covers_nothing() {
    assert_eq!(stroke(0.0, StrokeAlign::Centered).band(0.0), f32::INFINITY);
  }
}
