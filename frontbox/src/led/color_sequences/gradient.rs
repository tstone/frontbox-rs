use image::Rgba;

use crate::prelude::{ColorSequence, RgbaColor};

pub struct GradientStop {
  pub color: Rgba<u8>,
  pub position: f32, // 0.0..=1.0
}

impl GradientStop {
  /// position = 0.0..=1.0
  pub fn new(position: f32, color: Rgba<u8>) -> Self {
    GradientStop { color, position }
  }
}

pub struct Gradient {
  stops: Vec<GradientStop>, // must be sorted by position, ascending
}

impl Gradient {
  pub fn new(stops: Vec<GradientStop>) -> Self {
    Gradient { stops }
  }

  /// Even distribution of colors without complexity of multi-stop
  pub fn even(colors: Vec<Rgba<u8>>) -> Self {
    let len = colors.len();
    let stops = colors
      .into_iter()
      .enumerate()
      .map(|(i, color)| GradientStop {
        color,
        position: i as f32 / (len - 1).max(1) as f32,
      })
      .collect();
    Gradient { stops }
  }

  pub fn from_to(from: Rgba<u8>, to: Rgba<u8>) -> Self {
    Self::even(vec![from, to])
  }

  fn sample(&self, t: f32) -> Rgba<u8> {
    // find the two stops that straddle t
    let pair = self
      .stops
      .windows(2)
      .find(|w| t >= w[0].position && t <= w[1].position);

    match pair {
      Some(w) => {
        // windows(2) yields a slice of length 2; index directly to avoid
        // non-exhaustive slice pattern matching pitfalls.
        let a = &w[0];
        let b = &w[1];
        let span = b.position - a.position;
        let local_t = if span <= 0.0 {
          0.0
        } else {
          (t - a.position) / span
        };
        a.color.mix_with(b.color, local_t)
      }
      None => {
        // t is before the first stop or after the last
        if t <= self.stops[0].position {
          self.stops[0].color
        } else {
          self.stops[self.stops.len() - 1].color
        }
      }
    }
  }
}

impl ColorSequence for Gradient {
  fn render(&self, count: usize) -> Vec<Rgba<u8>> {
    (0..count)
      .map(|i| {
        let t = if count <= 1 {
          0.0
        } else {
          i as f32 / (count - 1) as f32
        };
        self.sample(t)
      })
      .collect()
  }
}
