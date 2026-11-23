use glam::Vec2;
use image::Rgba;
use palette::{FromColor, Hsv, Srgb};

use crate::animation::Lerp;
use crate::scene::{Color, NamedValue, Prop, Spread, Value, VariableType};

/// A type a `Prop` can hold
pub trait Animatable: Lerp + Clone + Default + PartialEq {
  /// What a variable it's read from has to be
  const VARIABLE_TYPE: VariableType;

  /// Reads a variable value. `None` if it's a different type.
  fn from_value(value: &Value) -> Option<Self>;

  /// Picks a value within a spread, using `rnd` (random numbers, each 0 to 1) for where in each range. `None` if the
  /// spread is a different type. A `Pool` never reaches this: it's read with `from_value`.
  fn from_spread(spread: &Spread, rnd: [f32; 3]) -> Option<Self>;

  /// Converts a point from a `Sweep`. `None` for types that aren't positions.
  fn from_point(point: Vec2) -> Option<Self>;

  /// A named value's prop. `None` if it's a different type.
  fn named_prop(named: &NamedValue) -> Option<&Prop<Self>>;
}

/// Maps `rnd`, from 0 to 1, into `low..=high`
fn rng_range((low, high): (f32, f32), rnd: f32) -> f32 {
  low + (high - low) * rnd
}

impl Animatable for f32 {
  const VARIABLE_TYPE: VariableType = VariableType::Number;

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

  fn named_prop(named: &NamedValue) -> Option<&Prop<Self>> {
    match named {
      NamedValue::Float(prop) => Some(prop),
      _ => None,
    }
  }
}

impl Animatable for Vec2 {
  const VARIABLE_TYPE: VariableType = VariableType::Vec2;

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

  fn named_prop(named: &NamedValue) -> Option<&Prop<Self>> {
    match named {
      NamedValue::Vec2(prop) => Some(prop),
      _ => None,
    }
  }
}

impl Animatable for Color {
  const VARIABLE_TYPE: VariableType = VariableType::Color;

  fn from_value(value: &Value) -> Option<Self> {
    match value {
      Value::Color(v) => Some(*v),
      _ => None,
    }
  }

  fn from_spread(spread: &Spread, [hue_rnd, _, _]: [f32; 3]) -> Option<Self> {
    match *spread {
      Spread::Color { hue } => {
        let picked = Hsv::new(rng_range(hue, hue_rnd), 1.0, 1.0);
        let (r, g, b) = Srgb::from_color(picked)
          .into_format::<u8>()
          .into_components();
        Some(Color(Rgba([r, g, b, 255])))
      }
      _ => None,
    }
  }

  fn from_point(_: Vec2) -> Option<Self> {
    None
  }

  fn named_prop(named: &NamedValue) -> Option<&Prop<Self>> {
    match named {
      NamedValue::Color(prop) => Some(prop),
      _ => None,
    }
  }
}

/// Whether a number counts as true: 0.5 and above does
fn is_true(number: f32) -> bool {
  number >= 0.5
}

impl Animatable for bool {
  const VARIABLE_TYPE: VariableType = VariableType::Bool;

  /// A `Bool`, or a number, which is true from 0.5 up
  fn from_value(value: &Value) -> Option<Self> {
    match value {
      Value::Bool(v) => Some(*v),
      Value::Float(v) => Some(is_true(*v)),
      Value::Int(v) => Some(is_true(*v as f32)),
      _ => None,
    }
  }

  /// A number picked within a `Float` spread, which is true from 0.5 up, so a wave from 0 to 1 is true for its upper half
  fn from_spread(spread: &Spread, rnd: [f32; 3]) -> Option<Self> {
    f32::from_spread(spread, rnd).map(is_true)
  }

  fn from_point(_: Vec2) -> Option<Self> {
    None
  }

  fn named_prop(named: &NamedValue) -> Option<&Prop<Self>> {
    match named {
      NamedValue::Bool(prop) => Some(prop),
      _ => None,
    }
  }
}

impl Lerp for Vec2 {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    self.lerp(*other, t)
  }
}
