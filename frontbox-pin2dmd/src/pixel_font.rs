use image;
use image::*;
use std::collections::HashMap;

use crate::*;

pub struct PixelFont {
  sprite_sheet: SpriteSheet,
  pub(crate) char_width: u16,
  pub(crate) char_height: u16,
  starting_char: u32,
  custom_char_widths: HashMap<char, u16>,
}

impl PixelFont {
  pub fn new(sprite_sheet: SpriteSheet, starting_char: char) -> Self {
    Self {
      char_width: sprite_sheet.sprite_width(),
      char_height: sprite_sheet.sprite_height(),
      starting_char: starting_char as u32,
      custom_char_widths: HashMap::new(),
      sprite_sheet,
    }
  }

  /// Set a custom width for a specific character (for variable width fonts)
  pub fn set_custom_char_width(&mut self, character: char, width: u16) {
    self.custom_char_widths.insert(character, width);
  }

  pub fn char(&self, c: char) -> ImageSprite {
    let char_code = c as u32;
    if char_code < self.starting_char {
      panic!("Character '{}' is not supported by this font", c);
    }
    let char_index = char_code - self.starting_char;
    let row = char_index / self.sprite_sheet.cols as u32;
    let col = char_index % self.sprite_sheet.cols as u32;

    self.sprite_sheet.image_at(row as u8, col as u8)
  }

  pub fn text(&self, text: String) -> ImageSprite {
    let text_width = text
      .chars()
      .map(|c| *self.custom_char_widths.get(&c).unwrap_or(&self.char_width))
      .sum::<u16>() as u32;

    let text_height = self.char_height as u32;
    let mut left_offset: i64 = 0;
    let mut result = RgbaImage::new(text_width, text_height);

    for c in text.chars() {
      let char_code = c as u32;
      if char_code < self.starting_char {
        continue; // skip unsupported characters
      }
      let char_index = char_code - self.starting_char;
      let row = char_index / self.sprite_sheet.cols as u32;
      let col = char_index % self.sprite_sheet.cols as u32;

      let sprite = self.sprite_sheet.image_at(row as u8, col as u8);
      let mut char_img = sprite.render().image;
      if self.custom_char_widths.contains_key(&c) {
        let char_width = *self.custom_char_widths.get(&c).unwrap();
        char_img = char_img.crop_imm(0, 0, char_width as u32, self.char_height as u32);
      }

      image::imageops::overlay(&mut result, &char_img, left_offset, 0);
      left_offset += *self.custom_char_widths.get(&c).unwrap_or(&self.char_width) as i64;
    }
    ImageSprite::new(DynamicImage::ImageRgba8(result))
  }
}

pub struct PixelFontBuilder {
  sprite_sheet: Option<SpriteSheet>,
  path: Option<&'static str>,
  rows: Option<u8>,
  cols: Option<u8>,
  starting_char: char,
  char_width: Option<u16>,
  custom_char_widths: HashMap<char, u16>,
}

impl PixelFontBuilder {
  pub fn new() -> Self {
    Self {
      sprite_sheet: None,
      starting_char: ' ',
      char_width: None,
      custom_char_widths: HashMap::new(),
      rows: None,
      cols: None,
      path: None,
    }
  }

  pub fn path(mut self, path: &'static str) -> Self {
    self.path = Some(path);
    self
  }

  pub fn sheet_layout(mut self, rows: u8, cols: u8) -> Self {
    self.rows = Some(rows);
    self.cols = Some(cols);
    self
  }

  pub fn sprite_sheet(mut self, sprite_sheet: SpriteSheet) -> Self {
    self.sprite_sheet = Some(sprite_sheet);
    self
  }

  pub fn default_char_width(mut self, width: u16) -> Self {
    self.char_width = Some(width);
    self
  }

  pub fn custom_char_width(mut self, character: char, pixel_width: u16) -> Self {
    self.custom_char_widths.insert(character, pixel_width);
    self
  }

  pub fn build(self) -> PixelFont {
    let sprite_sheet = if let Some(sprite_sheet) = self.sprite_sheet {
      sprite_sheet
    } else if let (Some(path), Some(rows), Some(cols)) = (self.path, self.rows, self.cols) {
      let sprite_sheet = SpriteSheet::new(path, rows, cols);
      sprite_sheet
    } else {
      panic!("Must provide either a SpriteSheet or path + layout");
    };

    let mut font = PixelFont::new(sprite_sheet, self.starting_char);
    for (character, width) in self.custom_char_widths {
      font.set_custom_char_width(character, width);
    }

    if let Some(default_width) = self.char_width {
      font.char_width = default_width;
    }

    font
  }
}
