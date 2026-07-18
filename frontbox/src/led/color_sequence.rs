use dyn_clone::DynClone;
use image::Rgba;

use crate::led::color_sequences::gradient::*;
use crate::led::color_sequences::pattern::*;
use crate::led::color_sequences::tile::*;

dyn_clone::clone_trait_object!(ColorSequence);

/// A description of a sequence of colors, given a of colors to generate
pub trait ColorSequence: DynClone + Send + Sync {
  fn base_render(&self, count: usize) -> Vec<Rgba<u8>>;
  fn rotation(&self) -> f32;
  fn reversed(&self) -> bool;

  fn render(&self, count: usize) -> Vec<Rgba<u8>> {
    let mut colors = self.base_render(count);
    util::rotate(self.rotation(), count, &mut colors);

    if self.reversed() {
      colors.reverse();
    }

    colors
  }
}

impl ColorSequence for Box<dyn ColorSequence> {
  fn base_render(&self, count: usize) -> Vec<Rgba<u8>> {
    (**self).base_render(count)
  }

  fn reversed(&self) -> bool {
    (**self).reversed()
  }

  fn rotation(&self) -> f32 {
    (**self).rotation()
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
    Pattern::new(colors)
  }

  /// A sequence of colors, applied once
  pub fn pattern_at(colors: Vec<Rgba<u8>>, index: u16) -> Pattern {
    Pattern::at(index, colors)
  }

  pub fn tile(colors: Vec<Rgba<u8>>) -> Tile {
    Tile::new(colors)
  }
}

pub(crate) mod util {
  use image::Rgba;

  pub fn rotate(degrees: f32, count: usize, colors: &mut Vec<Rgba<u8>>) {
    if degrees != 0.0 {
      let steps = (degrees.abs() / 360.0 * count as f32).round() as usize % count;

      // negative rotation = counterclockwise
      if degrees < 0.0 {
        colors.rotate_left(steps);
      } else {
        colors.rotate_right(steps);
      }
    }
  }
}
