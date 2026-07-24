use frontbox_canvas::*;
use image::DynamicImage;

use crate::LinearStitch;

pub struct OverflowText {
  pub(crate) glyph_images: Vec<DynamicImage>,
  pub(crate) ellipsis_img: DynamicImage,
  pub(crate) spacing: u8,
}

impl LayerGenerator for OverflowText {
  fn generate(&self, viewport: &Size<u32>) -> Layer {
    // early short circuit for performance (avoids a clone)
    let width = LinearStitch::horizontal_width(&self.glyph_images, self.spacing as u32);
    if width <= viewport.width {
      return Layer::top_left(LinearStitch::horizontal(
        &self.glyph_images,
        self.spacing as u32,
      ));
    }

    let mut glyphs = self.glyph_images.clone();
    // we already know it's too long, so start by dropping the last character
    let _ = glyphs.pop();

    // keep dropping the last character until it fits
    loop {
      let width = LinearStitch::horizontal_width(&glyphs, self.spacing as u32);
      if width < viewport.width {
        break;
      } else {
        let _ = glyphs.pop();
      }
    }

    Layer::top_left(LinearStitch::horizontal(&glyphs, self.spacing as u32))
  }
}
