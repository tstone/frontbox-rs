use glam::Vec2;
use image::Rgba;
use palette::{FromColor, Mix, Oklab, Srgb};
use serde::{Deserialize, Serialize};

use crate::animation::Lerp;
use crate::scene::{Color, ColorAdjust, Env, Node, Prop, Warp};

/// What a layer is drawn with
///
/// A paint is drawn in its own space, a unit box from `-0.5` to `0.5`. The layer stretches that box over its
/// shape's own bounds, so a paint fills its shape and moves and turns with it. With no shape, the box is the viewport.
pub type Paint = Node<PaintNode, ColorAdjust>;

impl Paint {
  /// The color at a point in the paint's box, before its warps
  pub(crate) fn color(&self, p: Vec2, env: &Env) -> Rgba<u8> {
    let Some(to_local) = self.to_local(env) else {
      return Rgba([0, 0, 0, 0]);
    };
    let color = self.base.color(to_local.transform_point2(p), env);
    self.adjust.iter().fold(color, |color, adjust| adjust.apply(color, env))
  }

  pub(crate) fn is_animated(&self) -> bool {
    self.base.is_animated()
      || self.warp.iter().any(Warp::is_animated)
      || self.adjust.iter().any(ColorAdjust::is_animated)
  }
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaintNode {
  Solid(Prop<Color>),
  /// Runs left to right across the paint's space, so `t` is `x + 0.5`. Past the ends, `t` clamps to the end colors.
  /// A vertical or diagonal gradient is a gradient with a `Rotate` warp.
  Gradient {
    /// In order of `at`
    stops: Vec<Stop>,
    #[serde(default)]
    space: ColorSpace,
  },
}

/// A color at a point along a gradient, from 0 to 1
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stop {
  pub at: f32,
  pub color: Prop<Color>,
}

impl Stop {
  pub fn new(at: f32, color: impl Into<Prop<Color>>) -> Self {
    Self { at, color: color.into() }
  }
}

/// How a gradient mixes between its colors
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorSpace {
  /// Perceptually even, so brightness doesn't dip between colors
  #[default]
  Oklab,
  /// Each channel separately. Red to blue dips to a dark purple in the middle.
  Rgb,
}

impl PaintNode {
  fn is_animated(&self) -> bool {
    match self {
      Self::Solid(color) => color.is_animated(),
      Self::Gradient { stops, .. } => stops.iter().any(|stop| stop.color.is_animated()),
    }
  }

  fn color(&self, p: Vec2, env: &Env) -> Rgba<u8> {
    match self {
      Self::Solid(color) => color.resolve(env).0,
      Self::Gradient { stops, space } => gradient(stops, *space, (p.x + 0.5).clamp(0.0, 1.0), env),
    }
  }
}

fn gradient(stops: &[Stop], space: ColorSpace, t: f32, env: &Env) -> Rgba<u8> {
  let (Some(first), Some(last)) = (stops.first(), stops.last()) else {
    return Rgba([0, 0, 0, 0]);
  };
  if t <= first.at {
    return first.color.resolve(env).0;
  }

  for pair in stops.windows(2) {
    let (a, b) = (&pair[0], &pair[1]);
    if t <= b.at {
      let span = b.at - a.at;
      let local = if span > 0.0 { (t - a.at) / span } else { 1.0 };
      return mix(a.color.resolve(env).0, b.color.resolve(env).0, local, space);
    }
  }

  last.color.resolve(env).0
}

fn mix(a: Rgba<u8>, b: Rgba<u8>, t: f32, space: ColorSpace) -> Rgba<u8> {
  match space {
    ColorSpace::Rgb => a.interpolate(&b, t),
    ColorSpace::Oklab => {
      let to_oklab = |c: Rgba<u8>| Oklab::from_color(Srgb::new(c[0], c[1], c[2]).into_format::<f32>());
      let mixed: Srgb<u8> = Srgb::from_color(to_oklab(a).mix(to_oklab(b), t)).into_format();
      Rgba([mixed.red, mixed.green, mixed.blue, a[3].interpolate(&b[3], t)])
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::scene::Params;
  use crate::scene::Warp;
  use crate::scene::value::tests::env;

  const RED: Rgba<u8> = Rgba([255, 0, 0, 255]);
  const BLUE: Rgba<u8> = Rgba([0, 0, 255, 255]);

  fn red_to_blue(space: ColorSpace) -> Paint {
    Paint::new(PaintNode::Gradient {
      stops: vec![Stop::new(0.0, Color(RED)), Stop::new(1.0, Color(BLUE))],
      space,
    })
  }

  fn color(paint: &Paint, x: f32, y: f32) -> Rgba<u8> {
    let params = Params::new();
    paint.color(Vec2::new(x, y), &env(0.0, &params))
  }

  #[test]
  fn gradient_runs_left_to_right_and_clamps() {
    let paint = red_to_blue(ColorSpace::Rgb);
    assert_eq!(color(&paint, -0.5, 0.0), RED);
    assert_eq!(color(&paint, 0.5, 0.0), BLUE);
    assert_eq!(color(&paint, -3.0, 0.0), RED);
    assert_eq!(color(&paint, 3.0, 0.0), BLUE);
    assert_eq!(color(&paint, 0.0, 0.0), Rgba([127, 0, 128, 255]));
  }

  #[test]
  fn oklab_stays_brighter_than_rgb_in_the_middle() {
    let rgb = color(&red_to_blue(ColorSpace::Rgb), 0.0, 0.0);
    let oklab = color(&red_to_blue(ColorSpace::Oklab), 0.0, 0.0);
    let sum = |c: Rgba<u8>| c[0] as u32 + c[1] as u32 + c[2] as u32;
    assert!(sum(oklab) > sum(rgb), "{oklab:?} vs {rgb:?}");
    assert_eq!(color(&red_to_blue(ColorSpace::Oklab), -0.5, 0.0), RED);
    assert_eq!(color(&red_to_blue(ColorSpace::Oklab), 0.5, 0.0), BLUE);
  }

  #[test]
  fn rotate_makes_a_gradient_vertical() {
    let paint = red_to_blue(ColorSpace::Rgb).warp(Warp::Rotate(90.0.into()));
    assert_eq!(color(&paint, 0.5, -0.5), RED);
    assert_eq!(color(&paint, -0.5, 0.5), BLUE);
  }

  #[test]
  fn stops_in_the_middle() {
    let paint = Paint::new(PaintNode::Gradient {
      stops: vec![
        Stop::new(0.25, Color(RED)),
        Stop::new(0.5, Color(Rgba([0, 255, 0, 255]))),
        Stop::new(0.75, Color(BLUE)),
      ],
      space: ColorSpace::Rgb,
    });
    assert_eq!(color(&paint, -0.4, 0.0), RED);
    assert_eq!(color(&paint, 0.0, 0.0), Rgba([0, 255, 0, 255]));
    assert_eq!(color(&paint, 0.4, 0.0), BLUE);
  }

  #[test]
  fn adjusts_apply_in_order_after_the_kind() {
    let mut paint = red_to_blue(ColorSpace::Rgb);
    paint.adjust.push(ColorAdjust::HueShift(120.0.into()));
    paint.adjust.push(ColorAdjust::HueShift((-60.0).into()));
    // red turned 60 degrees is yellow; blue turned 60 degrees is magenta
    assert_eq!(color(&paint, -0.5, 0.0), Rgba([255, 255, 0, 255]));
    assert_eq!(color(&paint, 0.5, 0.0), Rgba([255, 0, 255, 255]));
  }

  #[test]
  fn solid_ignores_position() {
    let paint = Paint::new(PaintNode::Solid(Color(RED).into()));
    assert_eq!(color(&paint, 100.0, -3.0), RED);
  }
}
