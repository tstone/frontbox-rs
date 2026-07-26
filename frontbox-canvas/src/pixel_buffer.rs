use frontbox::prelude::Rgba;
use image::ImageBuffer;

pub trait PixelBuffer {
  fn get_pixel_at(&mut self, x: u32, y: u32) -> &Rgba<u8>;
  fn put_pixel_at(&mut self, x: u32, y: u32, pixel: Rgba<u8>);
}

impl PixelBuffer for ImageBuffer<Rgba<u8>, Vec<u8>> {
  fn get_pixel_at(&mut self, x: u32, y: u32) -> &Rgba<u8> {
    self.get_pixel(x, y)
  }

  fn put_pixel_at(&mut self, x: u32, y: u32, pixel: Rgba<u8>) {
    self.put_pixel(x, y, pixel);
  }
}
