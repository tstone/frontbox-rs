use image::{DynamicImage, RgbaImage};

use crate::*;

pub struct PixelFontRenderable {
  pub(crate) char_sprites: Vec<RgbaImage>,
  pub(crate) char_widths: Vec<u8>,
  pub(crate) height: u32,
}

impl Renderable for PixelFontRenderable {
  fn render(&self, _parent: &FrameSize) -> RenderableImage {
    let total_width = self
      .char_widths
      .iter()
      .fold(0u32, |acc, w| acc + *w as u32 + 1)
      - 1; // -1 because the first glyph doesn't need an offset
    let mut result = RgbaImage::new(total_width, self.height);
    let mut left_offset: isize = 0;
    for (i, char_img) in self.char_sprites.iter().enumerate() {
      image::imageops::overlay(&mut result, char_img, left_offset as i64, 0);
      left_offset += self.char_widths[i] as isize + 1;
    }
    RenderableImage {
      image: DynamicImage::ImageRgba8(result),
      offset_x: 0,
      offset_y: 0,
    }
  }
}
