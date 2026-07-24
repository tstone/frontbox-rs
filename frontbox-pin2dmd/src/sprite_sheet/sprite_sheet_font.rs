use frontbox_canvas::*;
use image;
use image::*;
use std::collections::HashMap;

use crate::*;

pub struct SpriteSheetFont {
  sprite_sheet: SpriteSheet,
  pub(crate) char_width: u16,
  char_height: u32,
  starting_char: u32,
  custom_char_widths: HashMap<char, u16>,
}

impl SpriteSheetFont {
  pub fn new(sprite_sheet: SpriteSheet, starting_char: char) -> Self {
    let sprite_size = sprite_sheet.sprite_size();
    Self {
      char_width: sprite_size.width as u16,
      char_height: sprite_size.height,
      starting_char: starting_char as u32,
      custom_char_widths: HashMap::new(),
      sprite_sheet,
    }
  }

  /// By default, glyphs in a sprite sheet are assumed to be a fixed width matching the size of the sprites.
  /// This method allows setting a custom width for a specific character/glyph
  pub fn insert_custom_char_width(&mut self, character: char, width: u16) {
    self.custom_char_widths.insert(character, width);
  }

  /// A single character or glyph
  pub fn char_image(&self, c: char) -> Option<DynamicImage> {
    let char_code = c as u32;
    if char_code < self.starting_char {
      return None;
    }
    let char_index = char_code - self.starting_char;
    let row = char_index / self.sprite_sheet.cols as u32;
    let col = char_index % self.sprite_sheet.cols as u32;

    // TODO: drop extra pixels if custom character width is set
    Some(self.sprite_sheet.image_at(row as u8, col as u8))
  }

  /// Render a full string into sprites per character
  pub fn text(&self, text: impl Into<String>, spacing: u8) -> Layer {
    let text = text.into();
    let glyph_images = text
      .chars()
      .map(|c| self.char_image(c))
      .flatten()
      .collect::<Vec<_>>();

    Layer::top_left(LinearStitch::horizontal(&glyph_images, spacing as u32))
  }
}
