use glam::Vec2;
use image::Rgba;
use palette::{FromColor, Hsv, Srgb};

use crate::animation::Lerp;
use crate::scene::{Color, Spread, Value};

/// A type a `Prop` can hold
pub trait Animatable: Lerp + Clone + Default + PartialEq {
  /// Reads a param value. `None` if it's a different type.
  fn from_value(value: &Value) -> Option<Self>;

  /// Picks a value within a spread, using `rnd` (random numbers, each 0 to 1) for where in each range. `None` if the
  /// spread is a different type.
  fn from_spread(spread: &Spread, rnd: [f32; 3]) -> Option<Self>;

  /// Converts a point from a `Sweep`. `None` for types that aren't positions.
  fn from_point(point: Vec2) -> Option<Self>;
}

/// Maps `rnd`, from 0 to 1, into `low..=high`
fn rng_range((low, high): (f32, f32), rnd: f32) -> f32 {
  low + (high - low) * rnd
}

impl Animatable for f32 {
  /// A `Float`, or an `Int` as near as an `f32` holds it
  fn from_value(value: &Value) -> Option<Self> {
    match value {
      Value::Float(v) => Some(*v),
      Value::Int(v) => Some(*v as f32),
      _ => None,
    }
  }

  fn from_spread(spread: &Spread, [rnd, ..]: [f32; 3]) -> Option<Self> {
    match *spread {
      Spread::Float(low, high) => Some(rng_range((low, high), rnd)),
      _ => None,
    }
  }

  fn from_point(_: Vec2) -> Option<Self> {
    None
  }
}

impl Animatable for Vec2 {
  fn from_value(value: &Value) -> Option<Self> {
    match value {
      Value::Vec2(v) => Some(*v),
      _ => None,
    }
  }

  fn from_spread(spread: &Spread, [x_rnd, y_rnd, _]: [f32; 3]) -> Option<Self> {
    match *spread {
      Spread::Vec2 { x, y } => Some(Vec2::new(rng_range(x, x_rnd), rng_range(y, y_rnd))),
      _ => None,
    }
  }

  fn from_point(point: Vec2) -> Option<Self> {
    Some(point)
  }
}

impl Animatable for Color {
  fn from_value(value: &Value) -> Option<Self> {
    match value {
      Value::Color(v) => Some(*v),
      _ => None,
    }
  }

  fn from_spread(spread: &Spread, [hue_rnd, brightness_rnd, alpha_rnd]: [f32; 3]) -> Option<Self> {
    match *spread {
      Spread::Color { hue, brightness, alpha } => {
        let picked = Hsv::new(rng_range(hue, hue_rnd), 1.0, rng_range(brightness, brightness_rnd));
        let (r, g, b) = Srgb::from_color(picked).into_format::<u8>().into_components();
        Some(Color(Rgba([r, g, b, (rng_range(alpha, alpha_rnd) * 255.0).round() as u8])))
      }
      _ => None,
    }
  }

  fn from_point(_: Vec2) -> Option<Self> {
    None
  }
}

impl Lerp for Vec2 {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    self.lerp(*other, t)
  }
}
