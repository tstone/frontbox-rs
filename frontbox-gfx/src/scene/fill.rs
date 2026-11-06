use glam::Vec2;
use image::Rgba;
use palette::{FromColor, Mix, Oklab, Srgb};
use serde::{Deserialize, Serialize};

use crate::animation::Lerp;
use crate::scene::{Color, ColorAdjust, Env, Node, Prop, Transform};

/// What a layer is drawn with
///
/// A fill is drawn in its own space, a unit box from `-0.5` to `0.5`. The layer stretches that box over its
/// shape's own bounds, so a fill fills its shape and moves and turns with it. With no shape, the box is the viewport.
pub type Fill = Node<FillNode, ColorAdjust>;

impl Fill {
  /// The color at a point in the fill's box, before its transforms. `footprint` is how far the sample reaches from the
  /// point along each axis, in the same box.
  pub(crate) fn color(&self, p: Vec2, footprint: Vec2, env: &Env) -> Rgba<u8> {
    // the fill's own box runs around, and it's a point's width more than the box as the points reach past both ends
    let env = &Env { lap: env.lap.map(|_| 1.0 + 2.0 * footprint.x), ..*env };
    let p = self.on_lap(p, Vec2::ZERO, env);
    let Some(to_local) = self.to_local(p, env) else {
      return Rgba([0, 0, 0, 0]);
    };
    let footprint = to_local.matrix2.abs() * footprint;
    let color = self.base.color(to_local.transform_point2(p), footprint, env);
    self.adjust.iter().fold(color, |color, adjust| adjust.apply(color, env))
  }

  pub(crate) fn is_animated(&self) -> bool {
    self.base.is_animated()
      || self.transform.iter().any(Transform::is_animated)
      || self.adjust.iter().any(ColorAdjust::is_animated)
  }
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FillNode {
  Solid(Prop<Color>),
  /// Runs left to right across the fill's space, so `t` is `x + 0.5`. Past the ends, `t` clamps to the end colors.
  /// A vertical or diagonal gradient is a gradient with a `Rotate` transform.
  Gradient {
    /// In order of `at`
    stops: Vec<Stop>,
    #[serde(default)]
    space: ColorSpace,
  },
  /// An image stretched over the fill's space. Past its edges nothing is drawn, as an image has no colors to go on with.
  /// A `Sequence` runs along its middle row.
  Image {
    /// As it is in the [`Content`](crate::scene::Content) folder. An empty name draws nothing, for an image that's yet
    /// to be chosen.
    name: String,
    /// Which of an animated image's frames to draw, from 0, or the first if there's none. A fraction is dropped, so
    /// a value moving from 0 to the number of frames spends the same time on each, and past the last is the last.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    frame: Option<Prop<f32>>,
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

impl FillNode {
  /// An image, on its first frame
  pub fn image(name: impl Into<String>) -> Self {
    Self::Image { name: name.into(), frame: None }
  }

  fn is_animated(&self) -> bool {
    match self {
      Self::Solid(color) => color.is_animated(),
      Self::Gradient { stops, .. } => stops.iter().any(|stop| stop.color.is_animated()),
      Self::Image { frame, .. } => frame.as_ref().is_some_and(Prop::is_animated),
    }
  }

  pub(crate) fn image_name(&self) -> Option<&str> {
    match self {
      Self::Image { name, .. } if !name.is_empty() => Some(name),
      _ => None,
    }
  }

  fn color(&self, p: Vec2, footprint: Vec2, env: &Env) -> Rgba<u8> {
    match self {
      Self::Solid(color) => color.resolve(env).0,
      Self::Gradient { stops, space } => gradient(stops, *space, (p.x + 0.5).clamp(0.0, 1.0), env),
      Self::Image { name, frame } => env.images.get(name).map_or(Rgba([0, 0, 0, 0]), |image| {
        image.color(frame.as_ref().map_or(0.0, |frame| frame.resolve_whole(env)), p, footprint)
      }),
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
  use crate::scene::Transform;
  use crate::scene::value::tests::env;

  const RED: Rgba<u8> = Rgba([255, 0, 0, 255]);
  const BLUE: Rgba<u8> = Rgba([0, 0, 255, 255]);

  fn red_to_blue(space: ColorSpace) -> Fill {
    Fill::new(FillNode::Gradient {
      stops: vec![Stop::new(0.0, Color(RED)), Stop::new(1.0, Color(BLUE))],
      space,
    })
  }

  fn color(fill: &Fill, x: f32, y: f32) -> Rgba<u8> {
    let params = Params::new();
    fill.color(Vec2::new(x, y), Vec2::ZERO, &env(0.0, &params))
  }

  #[test]
  fn gradient_runs_left_to_right_and_clamps() {
    let fill = red_to_blue(ColorSpace::Rgb);
    assert_eq!(color(&fill, -0.5, 0.0), RED);
    assert_eq!(color(&fill, 0.5, 0.0), BLUE);
    assert_eq!(color(&fill, -3.0, 0.0), RED);
    assert_eq!(color(&fill, 3.0, 0.0), BLUE);
    assert_eq!(color(&fill, 0.0, 0.0), Rgba([127, 0, 128, 255]));
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
    let fill = red_to_blue(ColorSpace::Rgb).transform(Transform::Rotate(90.0.into()));
    assert_eq!(color(&fill, 0.5, -0.5), RED);
    assert_eq!(color(&fill, -0.5, 0.5), BLUE);
  }

  #[test]
  fn stops_in_the_middle() {
    let fill = Fill::new(FillNode::Gradient {
      stops: vec![
        Stop::new(0.25, Color(RED)),
        Stop::new(0.5, Color(Rgba([0, 255, 0, 255]))),
        Stop::new(0.75, Color(BLUE)),
      ],
      space: ColorSpace::Rgb,
    });
    assert_eq!(color(&fill, -0.4, 0.0), RED);
    assert_eq!(color(&fill, 0.0, 0.0), Rgba([0, 255, 0, 255]));
    assert_eq!(color(&fill, 0.4, 0.0), BLUE);
  }

  #[test]
  fn adjusts_apply_in_order_after_the_kind() {
    let mut fill = red_to_blue(ColorSpace::Rgb);
    fill.adjust.push(ColorAdjust::HueShift(120.0.into()));
    fill.adjust.push(ColorAdjust::HueShift((-60.0).into()));
    // red turned 60 degrees is yellow; blue turned 60 degrees is magenta
    assert_eq!(color(&fill, -0.5, 0.0), Rgba([255, 255, 0, 255]));
    assert_eq!(color(&fill, 0.5, 0.0), Rgba([255, 0, 255, 255]));
  }

  #[test]
  fn solid_ignores_position() {
    let fill = Fill::new(FillNode::Solid(Color(RED).into()));
    assert_eq!(color(&fill, 100.0, -3.0), RED);
  }
}
