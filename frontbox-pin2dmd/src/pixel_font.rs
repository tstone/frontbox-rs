use image;
use image::*;
use std::collections::HashMap;

use fast_protocol::Color;

use crate::SpriteSheet;

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
  pub fn set_character_width(&mut self, character: char, width: u16) {
    self.custom_char_widths.insert(character, width);
  }

  pub fn format_number(number: i64) -> String {
    let mut num_str = number.abs().to_string();
    let mut formatted = String::new();

    while num_str.len() > 3 {
      let chunk = num_str.split_off(num_str.len() - 3);
      formatted = format!(",{}{}", chunk, formatted);
    }
    formatted = format!("{}{}", num_str, formatted);

    if number < 0 {
      formatted = format!("-{}", formatted);
    }
    formatted
  }

  pub fn render_text(&self, text: &str) -> image::DynamicImage {
    let text_width = text
      .chars()
      .map(|c| *self.custom_char_widths.get(&c).unwrap_or(&self.char_width))
      .sum::<u16>() as u32;

    let text_height = self.char_height as u32;
    let mut left_offset: i64 = 0;
    let mut img = image::RgbaImage::new(text_width, text_height);

    for c in text.chars() {
      let char_code = c as u32;
      if char_code < self.starting_char {
        continue; // skip unsupported characters
      }
      let char_index = char_code - self.starting_char;
      let row = char_index / self.sprite_sheet.cols as u32;
      let col = char_index % self.sprite_sheet.cols as u32;

      let mut sprite = self.sprite_sheet.get_image_at(row as u16, col as u16);
      if self.custom_char_widths.contains_key(&c) {
        let char_width = *self.custom_char_widths.get(&c).unwrap();
        sprite = sprite.crop_imm(0, 0, char_width as u32, self.char_height as u32);
      }

      image::imageops::overlay(&mut img, &sprite.to_rgba8(), left_offset, 0);
      left_offset += *self.custom_char_widths.get(&c).unwrap_or(&self.char_width) as i64;
    }
    image::DynamicImage::ImageRgba8(img)
  }

  /// Render text, using the brightness of the original image to recolor it to the specified color
  pub fn render_text_recolor(&self, text: &str, color: Color) -> DynamicImage {
    let img = self.render_text(text).to_rgba8();
    let (width, height) = img.dimensions();
    let bytes = img.as_raw();
    let mut output = RgbaImage::new(width, height);

    for y in 0..height {
      for x in 0..width {
        let idx = (y * width + x) as usize * 4;
        let alpha = bytes[idx + 3] as f32 / 255.0;
        if alpha > 0.0 {
          let brightness = (bytes[idx] as f32 * 0.299
            + bytes[idx + 1] as f32 * 0.587
            + bytes[idx + 2] as f32 * 0.114)
            / 255.0;

          let pixel = Rgba([
            (color.r * brightness * 255.0).clamp(0.0, 255.0) as u8,
            (color.g * brightness * 255.0).clamp(0.0, 255.0) as u8,
            (color.b * brightness * 255.0).clamp(0.0, 255.0) as u8,
            (alpha * 255.0) as u8,
          ]);

          output.put_pixel(x, y, pixel);
        }
      }
    }
    DynamicImage::ImageRgba8(output)
  }

  /// Render text with a vertical gradient from top_color to bottom_color
  pub fn render_text_vgradient(
    &self,
    text: &str,
    top_color: Color,
    bottom_color: Color,
  ) -> DynamicImage {
    let img = self.render_text(text).to_rgba8();
    let (width, height) = img.dimensions();
    let bytes = img.as_raw();
    let mut output = RgbaImage::new(width, height);

    for y in 0..height {
      for x in 0..width {
        let idx = (y * width + x) as usize * 4;
        let alpha = bytes[idx + 3] as f32 / 255.0;
        if alpha > 0.0 {
          let brightness = (bytes[idx] as f32 * 0.299
            + bytes[idx + 1] as f32 * 0.587
            + bytes[idx + 2] as f32 * 0.114)
            / 255.0;

          let gradient_factor = y as f32 / height as f32;
          let r = top_color.r * (1.0 - gradient_factor) + bottom_color.r * gradient_factor;
          let g = top_color.g * (1.0 - gradient_factor) + bottom_color.g * gradient_factor;
          let b = top_color.b * (1.0 - gradient_factor) + bottom_color.b * gradient_factor;

          let pixel = Rgba([
            (r * brightness * 255.0).clamp(0.0, 255.0) as u8,
            (g * brightness * 255.0).clamp(0.0, 255.0) as u8,
            (b * brightness * 255.0).clamp(0.0, 255.0) as u8,
            (alpha * 255.0) as u8,
          ]);

          output.put_pixel(x, y, pixel);
        }
      }
    }
    DynamicImage::ImageRgba8(output)
  }

  /// Render text with a horizontal gradient from left_color to right_color
  pub fn render_text_hgradient(
    &self,
    text: &str,
    left_color: Color,
    right_color: Color,
  ) -> DynamicImage {
    let img = self.render_text(text).to_rgba8();
    let (width, height) = img.dimensions();
    let bytes = img.as_raw();
    let mut output = RgbaImage::new(width, height);

    for y in 0..height {
      for x in 0..width {
        let idx = (y * width + x) as usize * 4;
        let alpha = bytes[idx + 3] as f32 / 255.0;
        if alpha > 0.0 {
          let brightness = (bytes[idx] as f32 * 0.299
            + bytes[idx + 1] as f32 * 0.587
            + bytes[idx + 2] as f32 * 0.114)
            / 255.0;

          let gradient_factor = x as f32 / width as f32;
          let r = left_color.r * (1.0 - gradient_factor) + right_color.r * gradient_factor;
          let g = left_color.g * (1.0 - gradient_factor) + right_color.g * gradient_factor;
          let b = left_color.b * (1.0 - gradient_factor) + right_color.b * gradient_factor;

          let pixel = Rgba([
            (r * brightness * 255.0).clamp(0.0, 255.0) as u8,
            (g * brightness * 255.0).clamp(0.0, 255.0) as u8,
            (b * brightness * 255.0).clamp(0.0, 255.0) as u8,
            (alpha * 255.0) as u8,
          ]);

          output.put_pixel(x, y, pixel);
        }
      }
    }
    DynamicImage::ImageRgba8(output)
  }
}

pub struct PixelFontBuilder {
  sprite_sheet: Option<SpriteSheet>,
  path: Option<&'static str>,
  rows: Option<u16>,
  cols: Option<u16>,
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

  pub fn sheet_layout(mut self, rows: u16, cols: u16) -> Self {
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
      font.set_character_width(character, width);
    }

    if let Some(default_width) = self.char_width {
      font.char_width = default_width;
    }

    font
  }
}
