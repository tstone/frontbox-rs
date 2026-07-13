use crate::led::color_sequences::rgb_hsv::*;
use crate::prelude::*;

pub struct HueShift {
  pub degrees: f32,
  pub other: Box<dyn ColorSequence>,
}

impl ColorSequence for HueShift {
  fn render(&self, count: usize) -> Vec<Rgba<u8>> {
    self
      .other
      .render(count)
      .into_iter()
      .map(|color| shift_hue(color, self.degrees))
      .collect()
  }
}

fn shift_hue(color: Rgba<u8>, degrees: f32) -> Rgba<u8> {
  let Rgba([r, g, b, a]) = color;
  let (h, s, v) = rgb_to_hsv(r, g, b);
  let h = (h + degrees).rem_euclid(360.0);
  let (r, g, b) = hsv_to_rgb(h, s, v);
  Rgba([r, g, b, a])
}
