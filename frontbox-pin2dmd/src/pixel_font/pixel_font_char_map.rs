use std::collections::HashMap;

use frontbox::prelude::Rgba;
use frontbox_canvas::CanvasView;

use crate::{PixelFontMultiLineText, PixelFontOverflowText, PixelFontText, TextAlignment};

pub struct PixelFontCharacterMap {
  pub name: &'static str,
  pub height: u8,
  // key: code point/key code
  pub glyphs: HashMap<char, PixelFontGlyph>,
}

pub struct PixelFontGlyph {
  pub name: &'static str,
  pub width: u8,
  pub pixels: Vec<bool>,
}

impl PixelFontCharacterMap {
  /// Return a layer of renderable text
  pub fn text(
    &'static self,
    text: impl Into<String>,
    color: Rgba<u8>,
    spacing: u8,
    alignment: TextAlignment,
  ) -> PixelFontText {
    PixelFontText {
      text: text.into(),
      color,
      font: self,
      spacing,
      alignment,
    }
  }

  pub fn left_aligned(&'static self, text: impl Into<String>, color: Rgba<u8>) -> PixelFontText {
    self.text(text, color, 1, TextAlignment::Left)
  }

  pub fn center_aligned(&'static self, text: impl Into<String>, color: Rgba<u8>) -> PixelFontText {
    self.text(text, color, 1, TextAlignment::Centered)
  }

  pub fn right_aligned(&'static self, text: impl Into<String>, color: Rgba<u8>) -> PixelFontText {
    self.text(text, color, 1, TextAlignment::Right)
  }

  /// Same as `text` but will truncate the string with '…' if there is not enough space
  pub fn overflow_text(
    &'static self,
    text: impl Into<String>,
    color: Rgba<u8>,
    spacing: u8,
  ) -> PixelFontOverflowText {
    PixelFontOverflowText {
      text: text.into(),
      color,
      font: self,
      spacing,
    }
  }

  pub fn multi_line_text(
    &'static self,
    text: impl Into<String>,
    color: Rgba<u8>,
    spacing: u8,
  ) -> PixelFontMultiLineText {
    PixelFontMultiLineText {
      text: text.into(),
      color,
      font: self,
      spacing,
    }
  }

  pub fn glyph(&self, c: &char) -> Option<&PixelFontGlyph> {
    self.glyphs.get(c)
  }

  pub fn glyph_case_indeterminate(&self, c: &char) -> Option<&PixelFontGlyph> {
    let mut glyph = self.glyphs.get(&c);
    // check upper case if not found since most pixel fonts are only upper case
    if glyph.is_none() {
      glyph = self.glyphs.get(&c.to_ascii_uppercase())
    }
    glyph
  }

  pub fn render_char_image(&self, c: char, color: Rgba<u8>, canvas: &mut CanvasView) -> u8 {
    if let Some(glyph) = self.glyph_case_indeterminate(&c) {
      for (i, &on) in glyph.pixels.iter().enumerate() {
        if on {
          let x = (i % glyph.width as usize) as u32;
          let y = (i / glyph.width as usize) as u32;
          canvas.put_pixel(x, y, color);
        }
      }

      // return rendered width to maintain kerning
      return glyph.width;
    }
    0
  }
}
