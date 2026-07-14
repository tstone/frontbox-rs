use crate::led::color_sequences::rgb_hsv::*;
use crate::prelude::*;

pub struct Saturation {
  pub factor: f32, // 1.0 = unchanged, 0.0 = grayscale, >1.0 = oversaturated
  pub other: Box<dyn ColorSequence>,
}

impl ColorSequence for Saturation {
  fn render(&self, count: usize) -> Vec<Rgba<u8>> {
    self
      .other
      .render(count)
      .into_iter()
      .map(|Rgba([r, g, b, a])| {
        let (h, s, v) = rgb_to_hsv(r, g, b);
        let (r, g, b) = hsv_to_rgb(h, (s * self.factor).clamp(0.0, 1.0), v);
        Rgba([r, g, b, a])
      })
      .collect()
  }
}
