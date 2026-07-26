use std::collections::HashMap;
use std::path::Path;

use crate::{SpriteSheet, SpriteSheetFont};

pub struct SpriteSheetFontBuilder {
  sprite_sheet: Option<SpriteSheet>,
  path: Option<&'static Path>,
  rows: Option<u8>,
  cols: Option<u8>,
  starting_char: char,
  char_width: Option<u16>,
  custom_char_widths: HashMap<char, u16>,
}

impl SpriteSheetFontBuilder {
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

  pub fn path(mut self, path: impl Into<&'static Path>) -> Self {
    self.path = Some(path.into());
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

  pub fn build(self) -> SpriteSheetFont {
    let sprite_sheet = if let Some(sprite_sheet) = self.sprite_sheet {
      sprite_sheet
    } else if let (Some(path), Some(rows), Some(cols)) = (self.path, self.rows, self.cols) {
      SpriteSheet::new(path, rows, cols)
    } else {
      panic!("Must provide either a SpriteSheet or path + layout");
    };

    let mut font = SpriteSheetFont::new(sprite_sheet, self.starting_char);
    for (character, width) in self.custom_char_widths {
      font.insert_custom_char_width(character, width);
    }

    if let Some(default_width) = self.char_width {
      font.char_width = default_width;
    }

    font
  }
}

impl Default for SpriteSheetFontBuilder {
  fn default() -> Self {
    Self::new()
  }
}
