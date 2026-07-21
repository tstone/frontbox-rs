use image;
use image::*;
use std::collections::HashMap;

use crate::*;

pub struct SpriteSheetFont {
  sprite_sheet: SpriteSheet,
  pub(crate) char_width: u16,
  starting_char: u32,
  custom_char_widths: HashMap<char, u16>,
}

impl SpriteSheetFont {
  pub fn new(sprite_sheet: SpriteSheet, starting_char: char) -> Self {
    Self {
      char_width: sprite_sheet.sprite_width(),
      starting_char: starting_char as u32,
      custom_char_widths: HashMap::new(),
      sprite_sheet,
    }
  }

  /// Set a custom width for a specific character (for variable width fonts)
  pub fn set_custom_char_width(&mut self, character: char, width: u16) {
    self.custom_char_widths.insert(character, width);
  }

  pub fn char(&self, c: char) -> Asset {
    let char_code = c as u32;
    if char_code < self.starting_char {
      panic!("Character '{}' is not supported by this font", c);
    }
    let char_index = char_code - self.starting_char;
    let row = char_index / self.sprite_sheet.cols as u32;
    let col = char_index % self.sprite_sheet.cols as u32;

    self.sprite_sheet.image_at(row as u8, col as u8)
  }

  pub fn text(&self, text: impl Into<String>) -> SpriteSheetFontRenderable {
    let text = text.into();
    let text_width = text
      .chars()
      .map(|c| *self.custom_char_widths.get(&c).unwrap_or(&self.char_width))
      .sum::<u16>() as u32;

    let mut left_offset: isize = 0;

    let mut sprites = Vec::new();
    for c in text.chars() {
      let char_sprite = self.char(c);
      let char_width = *self.custom_char_widths.get(&c).unwrap_or(&self.char_width) as isize;
      sprites.push(char_sprite.left(left_offset));
      left_offset += char_width;
    }
    SpriteSheetFontRenderable {
      glyphs: sprites,
      glyph_widths: text
        .chars()
        .map(|c| *self.custom_char_widths.get(&c).unwrap_or(&self.char_width))
        .collect(),
      width: text_width,
    }
  }
}

pub struct SpriteSheetFontRenderable {
  glyphs: Vec<LeftOffsetRenderable>,
  glyph_widths: Vec<u16>,
  width: u32,
}

impl SpriteSheetFontRenderable {
  pub fn width(&self) -> u32 {
    self.width
  }

  /// split a word into a renderable per character
  pub fn split(&self, parent: &FrameSize) -> Vec<Asset> {
    let mut results = Vec::new();
    let mut left_offset: isize = 0;

    for (i, glyph) in self.glyphs.iter().enumerate() {
      let rendered = glyph.render(parent);
      results.push(Asset::new(rendered.image, left_offset, 0));
      left_offset += self.glyph_widths[i] as isize;
    }

    results
  }
}

impl Renderable for SpriteSheetFontRenderable {
  fn render(&self, parent: &FrameSize) -> RenderableImage {
    let mut result = RgbaImage::new(self.width, self.glyphs[0].render(parent).image.height());
    let mut left_offset: isize = 0;
    for (i, glyph) in self.glyphs.iter().enumerate() {
      let char_img = glyph.render(parent).image;
      image::imageops::overlay(&mut result, &char_img, left_offset as i64, 0);
      left_offset += self.glyph_widths[i] as isize;
    }
    RenderableImage {
      image: DynamicImage::ImageRgba8(result),
      offset_x: 0,
      offset_y: 0,
    }
  }
}

pub struct SpriteSheetFontBuilder {
  sprite_sheet: Option<SpriteSheet>,
  path: Option<String>,
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

  pub fn path(mut self, path: String) -> Self {
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

  pub fn build(self) -> SpriteSheetFont {
    let sprite_sheet = if let Some(sprite_sheet) = self.sprite_sheet {
      sprite_sheet
    } else if let (Some(path), Some(rows), Some(cols)) = (self.path, self.rows, self.cols) {
      let sprite_sheet = SpriteSheet::new(path, rows, cols);
      sprite_sheet
    } else {
      panic!("Must provide either a SpriteSheet or path + layout");
    };

    let mut font = SpriteSheetFont::new(sprite_sheet, self.starting_char);
    for (character, width) in self.custom_char_widths {
      font.set_custom_char_width(character, width);
    }

    if let Some(default_width) = self.char_width {
      font.char_width = default_width;
    }

    font
  }
}
