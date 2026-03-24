use crate::prelude::*;
use fast_protocol::Color;

/// Resolves LED conflicts by mixing all states in the list
pub struct BezierMixResolver;

impl BezierMixResolver {
  pub fn new() -> Self {
    Self
  }

  fn normalize_color(a: (u64, Color)) -> Color {
    if a.1 == Color::off() {
      Color::default()
    } else {
      a.1
    }
  }

  fn mix_pair(a: (u64, Color), b: (u64, Color)) -> Color {
    let c1 = Self::normalize_color(a);
    let c2 = Self::normalize_color(b);
    let cr = c1.mix(&c2, 0.5);

    if cr != Color::default() {
      return cr;
    } else {
      return Color::off();
    }
  }
}

impl LedResolver for BezierMixResolver {
  fn resolve(&mut self, _: &'static str, colors: Vec<(u64, Color)>) -> Color {
    if colors.len() == 0 {
      return Color::off();
    } else if colors.len() == 1 {
      return colors[0].1.clone();
    } else if colors.len() == 2 {
      Self::mix_pair(colors[0].clone(), colors[1].clone())
    } else {
      // if more than 2 colors, mix them in pairs recursively until we have one final color
      self.resolve(
        "",
        colors
          .windows(2)
          .map(|chunk| (0, Self::mix_pair(chunk[0].clone(), chunk[1].clone())))
          .collect(),
      )
    }
  }
}
