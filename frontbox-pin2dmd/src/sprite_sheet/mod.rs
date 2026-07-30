use std::path::Path;

use frontbox_canvas::Size;
use image::DynamicImage;

mod sprite_sheet_font;
mod sprite_sheet_font_builder;
mod sprite_sheet_text;

pub use sprite_sheet_font::*;
pub use sprite_sheet_font_builder::*;
pub use sprite_sheet_text::*;

/// Multiple sprite, within the same image file, laid out in a grid
#[derive(Debug, Clone)]
pub struct SpriteSheet {
  image: DynamicImage,
  pub rows: u8,
  pub cols: u8,
}

impl SpriteSheet {
  pub fn new(path: impl AsRef<Path>, rows: u8, cols: u8) -> Self {
    let path = path.as_ref();
    let image =
      image::open(path).unwrap_or_else(|_| panic!("Failed to load sprite sheet at {:?}", path));
    Self { image, rows, cols }
  }

  /// The width and height of an individual sprite
  pub fn sprite_size(&self) -> Size<u32> {
    let width = self.image.width() / self.cols as u32;
    let height = self.image.height() / self.rows as u32;
    Size::new(width, height)
  }

  pub fn image_at(&self, row: u8, col: u8) -> DynamicImage {
    let size = self.sprite_size();
    let x = col as u32 * size.width;
    let y = row as u32 * size.height;
    self.image.crop_imm(x, y, size.width, size.height)
  }
}
