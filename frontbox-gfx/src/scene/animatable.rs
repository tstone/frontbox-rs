use glam::Vec2;
use image::Rgba;
use palette::{FromColor, Hsv, Srgb};

use crate::animation::Lerp;
use crate::scene::{Color, Spread, Value};

/// A type a `Prop` can hold
pub trait Animatable: Lerp + Clone + Default + PartialEq {
  /// Reads a param value. `None` if it's a different type.
  fn from_value(value: &Value) -> Option<Self>;

  /// Picks a value within a spread, using `rolls` (random numbers, each 0 to 1) for where in each range. `None` if the
  /// spread is a different type.
  fn from_spread(spread: &Spread, rolls: [f32; 3]) -> Option<Self>;

  /// Converts a point from a `Sweep`. `None` for types that aren't positions.
  fn from_point(point: Vec2) -> Option<Self>;
}

fn between((low, high): (f32, f32), roll: f32) -> f32 {
  low + (high - low) * roll
}

impl Animatable for f32 {
  fn from_value(value: &Value) -> Option<Self> {
    match value {
      Value::Float(v) => Some(*v),
      _ => None,
    }
  }

  fn from_spread(spread: &Spread, [roll, ..]: [f32; 3]) -> Option<Self> {
    match *spread {
      Spread::Float(low, high) => Some(between((low, high), roll)),
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

  fn from_spread(spread: &Spread, [x_roll, y_roll, _]: [f32; 3]) -> Option<Self> {
    match *spread {
      Spread::Vec2 { x, y } => Some(Vec2::new(between(x, x_roll), between(y, y_roll))),
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

  fn from_spread(spread: &Spread, [hue_roll, brightness_roll, alpha_roll]: [f32; 3]) -> Option<Self> {
    match *spread {
      Spread::Color { hue, brightness, alpha } => {
        let picked = Hsv::new(between(hue, hue_roll), 1.0, between(brightness, brightness_roll));
        let (r, g, b) = Srgb::from_color(picked).into_format::<u8>().into_components();
        Some(Color(Rgba([r, g, b, (between(alpha, alpha_roll) * 255.0).round() as u8])))
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
