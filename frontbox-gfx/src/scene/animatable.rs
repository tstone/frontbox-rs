use glam::Vec2;

use crate::animation::Lerp;
use crate::scene::{Color, Value};

/// A type a `Prop` can hold
pub trait Animatable: Lerp + Clone + Default {
  fn from_value(value: &Value) -> Option<Self>;
}

impl Animatable for f32 {
  fn from_value(value: &Value) -> Option<Self> {
    match value {
      Value::Float(v) => Some(*v),
      _ => None,
    }
  }
}

impl Animatable for Vec2 {
  fn from_value(value: &Value) -> Option<Self> {
    match value {
      Value::Vec2(v) => Some(*v),
      _ => None,
    }
  }
}

impl Animatable for Color {
  fn from_value(value: &Value) -> Option<Self> {
    match value {
      Value::Color(v) => Some(*v),
      _ => None,
    }
  }
}

impl Lerp for Vec2 {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    self.lerp(*other, t)
  }
}
