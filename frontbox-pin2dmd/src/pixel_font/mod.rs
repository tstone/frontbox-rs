mod pixel_font_char_map;
mod pixel_font_renderable;
mod sigi_5px_bold;
mod sigi_5px_condensed_bold;
mod sigi_5px_condensed_regular;
mod sigi_5px_regular;
mod sigi_7px_bold;
mod sigi_7px_regular;
mod symbols_5px_regular;

pub use pixel_font_char_map::*;
pub use pixel_font_renderable::*;
pub use sigi_5px_bold::*;
pub use sigi_5px_condensed_bold::*;
pub use sigi_5px_condensed_regular::*;
pub use sigi_5px_regular::*;
pub use sigi_7px_bold::*;
pub use sigi_7px_regular::*;
pub use symbols_5px_regular::*;

use image::{ImageBuffer, Rgba, RgbaImage};

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

  pub fn char(&self, c: char, color: Rgba<u8>) -> Option<PixelFontRenderable> {
    let mut glyph = self.char_map.glyphs.get(&c);

    // check upper case if not found
    if glyph.is_none() {
      glyph = self.char_map.glyphs.get(&c.to_ascii_uppercase())
    }

    if let Some(glyph) = glyph {
      let mut sprite: RgbaImage = ImageBuffer::new(glyph.width as u32, self.char_map.height as u32);
      for (i, &on) in glyph.pixels.iter().enumerate() {
        if on {
          let x = (i % glyph.width as usize) as u32;
          let y = (i / glyph.width as usize) as u32;
          sprite.put_pixel(x, y, color);
        }
      }

      return Some(PixelFontRenderable {
        char_sprites: vec![sprite],
        char_widths: vec![glyph.width],
        height: self.char_map.height as u32,
      });
    }
    None
  }

  pub fn text(&self, text: impl Into<String>, color: Rgba<u8>) -> PixelFontRenderable {
    let mut sprites: Vec<RgbaImage> = Vec::new();
    let mut widths: Vec<u8> = Vec::new();

    for c in text.into().chars() {
      let mut glyph = self.char_map.glyphs.get(&c);
      // check upper case if not found
      if glyph.is_none() {
        glyph = self.char_map.glyphs.get(&c.to_ascii_uppercase())
      }

      if let Some(glyph) = glyph {
        let mut sprite: RgbaImage =
          ImageBuffer::new(glyph.width as u32, self.char_map.height as u32);
        for (i, &on) in glyph.pixels.iter().enumerate() {
          if on {
            let x = (i % glyph.width as usize) as u32;
            let y = (i / glyph.width as usize) as u32;
            sprite.put_pixel(x, y, color);
          }
        }

        widths.push(glyph.width);
        sprites.push(sprite);
      }
    }

    PixelFontRenderable {
      height: self.char_map.height as u32,
      char_sprites: sprites,
      char_widths: widths,
    }
  }
}
