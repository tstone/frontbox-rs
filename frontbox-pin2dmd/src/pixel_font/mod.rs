mod pixel_font_char_map;
mod sigi_5px_bold;
mod sigi_5px_condensed_bold;
mod sigi_5px_condensed_regular;
mod sigi_5px_regular;
mod sigi_7px_bold;
mod sigi_7px_regular;
mod symbols_5px_regular;

use std::sync::LazyLock;

use frontbox_canvas::Layer;
pub use pixel_font_char_map::*;

use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};

use crate::{LinearStitch, OverflowText};

pub static SIGI_5PX_BOLD: LazyLock<PixelFont> =
  LazyLock::new(|| PixelFont::new(&sigi_5px_bold::SIGI_BOLD_5PX_FONT));
pub static SIGI_5PX_CONDENSED_BOLD: LazyLock<PixelFont> =
  LazyLock::new(|| PixelFont::new(&sigi_5px_condensed_bold::SIGI_CONDENSED_BOLD_5PX_FONT));
pub static SIGI_5PX_CONDENSED_REGULAR: LazyLock<PixelFont> =
  LazyLock::new(|| PixelFont::new(&sigi_5px_condensed_regular::SIGI_CONDENSED_REGULAR_5PX_FONT));
pub static SIGI_5PX_REGULAR: LazyLock<PixelFont> =
  LazyLock::new(|| PixelFont::new(&sigi_5px_regular::SIGI_REGULAR_5PX_FONT));
pub static SIGI_7PX_REGULAR: LazyLock<PixelFont> =
  LazyLock::new(|| PixelFont::new(&sigi_7px_regular::SIGI_REGULAR_7PX_FONT));
pub static SIGI_7PX_BOLD: LazyLock<PixelFont> =
  LazyLock::new(|| PixelFont::new(&sigi_7px_bold::SIGI_BOLD_7PX_FONT));
pub static SYMBOLS_5PX_REGULAR: LazyLock<PixelFont> =
  LazyLock::new(|| PixelFont::new(&symbols_5px_regular::SYMBOLS_5PX_REGULAR));

pub struct PixelFont {
  char_map: &'static PixelFontCharacterMap,
}

impl PixelFont {
  pub fn new(char_map: &'static PixelFontCharacterMap) -> Self {
    Self { char_map }
  }

  pub fn height(&self) -> u8 {
    self.char_map.height
  }

  pub fn char_image(&self, c: char, color: Rgba<u8>) -> Option<DynamicImage> {
    let mut glyph = self.char_map.glyphs.get(&c);

    // check upper case if not found
    if glyph.is_none() {
      glyph = self.char_map.glyphs.get(&c.to_ascii_uppercase())
    }

    if let Some(glyph) = glyph {
      let mut buffer: RgbaImage = ImageBuffer::new(glyph.width as u32, self.char_map.height as u32);
      for (i, &on) in glyph.pixels.iter().enumerate() {
        if on {
          let x = (i % glyph.width as usize) as u32;
          let y = (i / glyph.width as usize) as u32;
          buffer.put_pixel(x, y, color);
        }
      }

      Some(DynamicImage::ImageRgba8(buffer))
    } else {
      None
    }
  }

  pub fn text(&self, text: impl Into<String>, color: Rgba<u8>, spacing: u8) -> Layer {
    let glyph_images = text
      .into()
      .chars()
      .map(|c| self.char_image(c, color))
      .flatten()
      .collect::<Vec<_>>();

    Layer::top_left(LinearStitch::horizontal(&glyph_images, spacing as u32))
  }

  /// Render text that fits within a given region, truncating with "..." if it is too long
  pub fn overflow_text(
    &self,
    text: impl Into<String>,
    color: Rgba<u8>,
    spacing: u8,
  ) -> OverflowText {
    let glyph_images = text
      .into()
      .chars()
      .map(|c| self.char_image(c, color))
      .flatten()
      .collect::<Vec<_>>();

    OverflowText {
      glyph_images,
      ellipsis_img: self.char_image('…', color).unwrap(),
      spacing,
    }
  }
}
