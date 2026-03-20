pub struct SpriteSheet {
  image: image::DynamicImage,
  pub rows: u16,
  pub cols: u16,
}

impl SpriteSheet {
  pub fn new(path: &'static str, rows: u16, cols: u16) -> Self {
    let image =
      image::open(path).unwrap_or_else(|_| panic!("Failed to load sprite sheet at {}", path));
    Self { image, rows, cols }
  }

  pub fn get_image_at(&self, row: u16, col: u16) -> image::DynamicImage {
    let sprite_width = self.image.width() / self.cols as u32;
    let sprite_height = self.image.height() / self.rows as u32;
    let x = col as u32 * sprite_width;
    let y = row as u32 * sprite_height;
    self.image.crop_imm(x, y, sprite_width, sprite_height)
  }

  pub fn sprite_width(&self) -> u16 {
    self.image.width() as u16 / self.cols
  }

  pub fn sprite_height(&self) -> u16 {
    self.image.height() as u16 / self.rows
  }
}
