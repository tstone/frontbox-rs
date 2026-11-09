use glam::Vec2;
use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::animation::Lerp;
use crate::scene::{Color, ColorAdjust, Env, Node, Prop, Transform};

/// The colors a layer is drawn with
///
/// A fill is defined over a unit box from `-0.5` to `0.5`, which is stretched over the shape's local bounds (or the
/// viewport, with no shape). So a fill always fits its shape, and moves and rotates with it.
pub type Fill = Node<FillNode, ColorAdjust>;

impl Fill {
  /// The color at a point in the unit box. `footprint` is the sample's radius along each axis, in unit box units.
  pub(crate) fn color(&self, p: Vec2, footprint: Vec2, env: &Env) -> Rgba<u8> {
    // on a Sequence, the lap is the unit box plus one sample's width, as samples reach half a width past each end
    let env = &Env { lap: env.lap.map(|_| 1.0 + 2.0 * footprint.x), ..*env };
    let p = self.on_lap(p, Vec2::ZERO, env);
    let Some(to_local) = self.to_local(Vec2::ZERO, env) else {
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

/// The base content of a fill
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FillNode {
  Solid(Prop<Color>),
  /// Runs left to right. Add a `Rotate` transform for other directions, or to move where a `Conical` one starts.
  Gradient {
    /// Sorted by `at`
    stops: Vec<Stop>,
    #[serde(default)]
    style: GradientStyle,
  },
  /// An image stretched over the unit box, and transparent outside it. On a `Sequence`, samples run along its middle
  /// row.
  Image {
    /// Path within the [`Content`](crate::scene::Content) folder. Empty draws nothing, for an image not chosen yet.
    name: String,
    /// Frame index for an animated image, 0 if not set. Truncated to a whole number, so animating it from 0 to the frame
    /// count gives each frame equal time. Clamped to the last frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    frame: Option<Prop<f32>>,
  },
}

/// A gradient color stop. `at` runs from 0 to 1.
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

/// How a gradient maps across the unit box
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GradientStyle {
  /// Left to right, holding the end colors past either end
  #[default]
  Linear,
  /// Out from the center, reaching 1 at the middle of each edge of the box and holding the last color past that
  Radial,
  /// Clockwise around the center, starting at 3 o'clock. 0 and 1 meet there, so there's a hard edge unless their colors
  /// match.
  Conical,
}

impl GradientStyle {
  /// Position along the gradient, 0 to 1, of a point in the unit box
  fn at(self, p: Vec2) -> f32 {
    match self {
      Self::Linear => (p.x + 0.5).clamp(0.0, 1.0),
      Self::Radial => (p.length() * 2.0).clamp(0.0, 1.0),
      Self::Conical => (p.y.atan2(p.x) / std::f32::consts::TAU).rem_euclid(1.0),
    }
  }
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
      Self::Gradient { stops, style } => gradient(stops, style.at(p), env),
      Self::Image { name, frame } => env.images.get(name).map_or(Rgba([0, 0, 0, 0]), |image| {
        image.color(frame.as_ref().map_or(0.0, |frame| frame.resolve_whole(env)), p, footprint)
      }),
    }
  }
}

fn gradient(stops: &[Stop], t: f32, env: &Env) -> Rgba<u8> {
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
      return a.color.resolve(env).0.interpolate(&b.color.resolve(env).0, local);
    }
  }

  last.color.resolve(env).0
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::scene::Params;
  use crate::scene::Transform;
  use crate::scene::value::tests::env;

  const RED: Rgba<u8> = Rgba([255, 0, 0, 255]);
  const BLUE: Rgba<u8> = Rgba([0, 0, 255, 255]);

  fn red_to_blue() -> Fill {
    styled(GradientStyle::Linear)
  }

  fn styled(style: GradientStyle) -> Fill {
    Fill::new(FillNode::Gradient {
      stops: vec![Stop::new(0.0, Color(RED)), Stop::new(1.0, Color(BLUE))],
      style,
    })
  }

  fn color(fill: &Fill, x: f32, y: f32) -> Rgba<u8> {
    let params = Params::new();
    fill.color(Vec2::new(x, y), Vec2::ZERO, &env(0.0, &params))
  }

  #[test]
  fn gradient_runs_left_to_right_and_clamps() {
    let fill = red_to_blue();
    assert_eq!(color(&fill, -0.5, 0.0), RED);
    assert_eq!(color(&fill, 0.5, 0.0), BLUE);
    assert_eq!(color(&fill, -3.0, 0.0), RED);
    assert_eq!(color(&fill, 3.0, 0.0), BLUE);
    assert_eq!(color(&fill, 0.0, 0.0), Rgba([127, 0, 128, 255]));
  }

  #[test]
  fn a_radial_gradient_runs_out_from_the_middle() {
    let fill = styled(GradientStyle::Radial);
    assert_eq!(color(&fill, 0.0, 0.0), RED);
    assert_eq!(color(&fill, 0.5, 0.0), BLUE);
    assert_eq!(color(&fill, 0.0, -0.5), BLUE);
    assert_eq!(color(&fill, 0.25, 0.0), color(&fill, 0.0, -0.25));
    assert_eq!(color(&fill, 0.25, 0.0), Rgba([127, 0, 128, 255]));
    assert_eq!(color(&fill, 0.5, 0.5), BLUE);
  }

  #[test]
  fn a_conical_gradient_runs_clockwise_from_the_right() {
    let fill = styled(GradientStyle::Conical);
    assert_eq!(color(&fill, 0.5, 0.0), RED);
    assert_eq!(color(&fill, 0.0, 0.5), Rgba([191, 0, 64, 255]));
    assert_eq!(color(&fill, -0.5, 0.0), Rgba([127, 0, 128, 255]));
    assert_eq!(color(&fill, 0.0, -0.5), Rgba([64, 0, 191, 255]));
    assert_eq!(color(&fill, 0.5, -0.0001), BLUE);
  }

  #[test]
  fn rotate_turns_where_a_conical_gradient_starts() {
    let fill = styled(GradientStyle::Conical).transform(Transform::Rotate(90.0.into()));
    // just past the start, as exactly on it could land either side of where the ends meet
    assert_eq!(color(&fill, -0.0001, 0.5), RED);
  }

  #[test]
  fn rotate_makes_a_gradient_vertical() {
    let fill = red_to_blue().transform(Transform::Rotate(90.0.into()));
    assert_eq!(color(&fill, 0.5, -0.5), RED);
    assert_eq!(color(&fill, -0.5, 0.5), BLUE);
  }

  #[test]
  fn a_fill_moved_then_rotated_turns_around_its_own_middle() {
    let fill = red_to_blue()
      .transform(Transform::Translate(Vec2::new(1.0, 0.0).into()))
      .transform(Transform::Rotate(90.0.into()));
    assert_eq!(color(&fill, 1.0, -0.5), RED);
    assert_eq!(color(&fill, 1.0, 0.5), BLUE);
  }

  #[test]
  fn stops_in_the_middle() {
    let fill = Fill::new(FillNode::Gradient {
      stops: vec![
        Stop::new(0.25, Color(RED)),
        Stop::new(0.5, Color(Rgba([0, 255, 0, 255]))),
        Stop::new(0.75, Color(BLUE)),
      ],
      style: GradientStyle::Linear,
    });
    assert_eq!(color(&fill, -0.4, 0.0), RED);
    assert_eq!(color(&fill, 0.0, 0.0), Rgba([0, 255, 0, 255]));
    assert_eq!(color(&fill, 0.4, 0.0), BLUE);
  }

  #[test]
  fn adjusts_apply_in_order_after_the_kind() {
    let mut fill = red_to_blue();
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
