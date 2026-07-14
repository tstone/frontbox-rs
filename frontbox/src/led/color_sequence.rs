use image::Rgba;

use crate::led::color_sequences::gamma::*;
use crate::led::color_sequences::gradient::*;
use crate::led::color_sequences::hue_shift::*;
use crate::led::color_sequences::invert::*;
use crate::led::color_sequences::pattern::*;
use crate::led::color_sequences::reverse::*;
use crate::led::color_sequences::rotate::*;
use crate::led::color_sequences::saturation::*;
use crate::led::color_sequences::tile::*;

/// A description of a sequence of colors, given a of colors to generate
pub trait ColorSequence {
  fn render(&self, count: usize) -> Vec<Rgba<u8>>;

  fn reverse(self) -> Reverse
  where
    Self: Sized + 'static,
  {
    Reverse {
      other: Box::new(self),
    }
  }

  fn pattern_at(self, index: u16, colors: Vec<Rgba<u8>>) -> Pattern
  where
    Self: Sized + 'static,
  {
    Pattern {
      seq: colors,
      index,
      other: Some(Box::new(self)),
    }
  }

  fn rotated_left(self, degrees: f32) -> Rotate
  where
    Self: Sized + 'static,
  {
    Rotate {
      direction: Rotation::CounterClockwise,
      degrees,
      other: Box::new(self),
    }
  }

  fn rotated_right(self, degrees: f32) -> Rotate
  where
    Self: Sized + 'static,
  {
    Rotate {
      direction: Rotation::Clockwise,
      degrees,
      other: Box::new(self),
    }
  }

  fn invert(self) -> Invert
  where
    Self: Sized + 'static,
  {
    Invert {
      other: Box::new(self),
    }
  }

  fn hue_shift(self, degrees: f32) -> HueShift
  where
    Self: Sized + 'static,
  {
    HueShift {
      degrees,
      other: Box::new(self),
    }
  }

  fn brightness(self, value: f32) -> Gamma
  where
    Self: Sized + 'static,
  {
    Gamma {
      value,
      other: Box::new(self),
    }
  }

  fn saturation(self, factor: f32) -> Saturation
  where
    Self: Sized + 'static,
  {
    Saturation {
      factor,
      other: Box::new(self),
    }
  }
}

pub struct Colors;

impl Colors {
  /// Smoothly fade all colors given. Dynamically resizes based on LED count.
  pub fn gradient(colors: Vec<Rgba<u8>>) -> Gradient {
    Gradient::even(colors)
  }

  /// Smoothly fade colors between given points. Dynamically resizes based on LED count.
  pub fn multi_gradient(stops: Vec<GradientStop>) -> Gradient {
    Gradient::new(stops)
  }

  /// A sequence of colors, applied once
  pub fn pattern(colors: Vec<Rgba<u8>>) -> Pattern {
    Pattern {
      seq: colors,
      index: 0,
      other: None,
    }
  }

  pub fn tile(colors: Vec<Rgba<u8>>) -> Tile {
    Tile { seq: colors }
  }
}
