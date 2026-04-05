use crate::prelude::*;
use fast_protocol::Color;

/// Resolves LED conflicts by mixing all states in the list
pub struct BezierMixResolver;

impl BezierMixResolver {
  pub fn resolve(colors: Vec<Color>) -> Color {
    if colors.len() == 0 {
      return Color::default();
    } else if colors.len() == 1 {
      return colors[0].clone();
    } else if colors.len() == 2 {
      colors[0].clone().mix(&colors[1], 0.5)
    } else {
      // if more than 2 colors, mix them in pairs recursively until we have one final color
      Self::resolve(
        colors
          .windows(2)
          .map(|chunk| chunk[0].mix(&chunk[1], 0.5))
          .collect(),
      )
    }
  }
}
