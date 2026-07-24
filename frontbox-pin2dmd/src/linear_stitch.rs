use image::*;

pub struct LinearStitch;

impl LinearStitch {
  /// Calculate the total horizontal width of a list of images, switched together horizontally.
  pub fn horizontal_width(images: &Vec<DynamicImage>, spacing: u32) -> u32 {
    images.iter().map(|img| img.width() + spacing).sum::<u32>() - spacing
  }

  /// Combine a list of images horizontally into a single image
  pub fn horizontal(images: &Vec<DynamicImage>, spacing: u32) -> DynamicImage {
    let width = Self::horizontal_width(&images, spacing);
    let max_height = images.iter().map(|img| img.height()).max().unwrap_or(0);

    let mut buffer = RgbaImage::new(width, max_height);
    let mut left_offset = 0;

    for img in images {
      let char_width = img.width() + spacing as u32;
      imageops::overlay(&mut buffer, img, left_offset as i64, 0);
      left_offset += char_width;
    }

    DynamicImage::ImageRgba8(buffer)
  }

  // TODO: vertical
}
