use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};

use crate::{FrameSize, PixelFontCharacterMap, Renderable, RenderableImage};

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

pub struct PixelFontRenderable {
  char_sprites: Vec<RgbaImage>,
  char_widths: Vec<u8>,
  height: u32,
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
