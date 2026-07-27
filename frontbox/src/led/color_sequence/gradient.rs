use image::Rgba;

use crate::animation::Lerp;
use crate::led::color_sequence::Extent;
use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct GradientStop {
  pub color: Rgba<u8>,
  pub position: Extent<u16>, // 0.0..=1.0
}

impl GradientStop {
  /// position = 0.0..=1.0
  pub fn new(position: impl Into<Extent<u16>>, color: Rgba<u8>) -> Self {
    GradientStop {
      color,
      position: position.into(),
    }
  }
}

impl Lerp for GradientStop {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    GradientStop {
      color: self.color.interpolate(&other.color, t),
      position: self.position.interpolate(&other.position, t),
    }
  }
}

pub fn render(stops: &Vec<GradientStop>, length: u16) -> Vec<Rgba<u8>> {
  (0..length)
    .map(|i| {
      let t = if length <= 1 {
        0.0
      } else {
        i as f32 / (length - 1) as f32
      };
      sample(stops, length, t)
    })
    .collect()
}

fn sample(stops: &Vec<GradientStop>, length: u16, t: f32) -> Rgba<u8> {
  // find the two stops that straddle t
  let pair = stops
    .windows(2)
    .find(|w| t >= w[0].position.to_relative(length) && t <= w[1].position.to_relative(length));

  match pair {
    Some(w) => {
      // windows(2) yields a slice of length 2; index directly to avoid
      // non-exhaustive slice pattern matching pitfalls.
      let a = &w[0];
      let b = &w[1];
      let a_pos = a.position.to_relative(length);
      let b_pos = b.position.to_relative(length);

      let span = b_pos - a_pos;
      let local_t = if span <= 0.0 { 0.0 } else { (t - a_pos) / span };
      a.color.mix_with(b.color, local_t)
    }
    None => {
      // t is before the first stop or after the last
      if t <= stops[0].position.to_relative(length) {
        stops[0].color
      } else {
        stops[stops.len() - 1].color
      }
    }
  }
}
