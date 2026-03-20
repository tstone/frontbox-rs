use crate::Asset;

/// Multiple sprite, within the same image file, layed out in a grid
#[derive(Debug, Clone)]
pub struct SpriteSheet {
  image: image::DynamicImage,
  pub path: &'static str,
  pub rows: u8,
  pub cols: u8,
}

impl SpriteSheet {
  pub fn new(path: &'static str, rows: u8, cols: u8) -> Self {
    let image =
      image::open(path).unwrap_or_else(|_| panic!("Failed to load sprite sheet at {}", path));
    Self {
      path,
      image,
      rows,
      cols,
    }
  }

  pub fn sprite_width(&self) -> u16 {
    self.image.width() as u16 / self.cols as u16
  }

  pub fn sprite_height(&self) -> u16 {
    self.image.height() as u16 / self.rows as u16
  }

  pub fn image_at(&self, row: u8, col: u8) -> Asset {
    let sprite_width = self.image.width() / self.cols as u32;
    let sprite_height = self.image.height() / self.rows as u32;
    let x = col as u32 * sprite_width;
    let y = row as u32 * sprite_height;
    let sprite = self.image.crop_imm(x, y, sprite_width, sprite_height);
    Asset::image(sprite)
  }
}
