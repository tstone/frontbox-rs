use fast_protocol::Color;

pub struct Layer {
  pub width: usize,
  pub height: usize,
  pub pixels: Vec<u8>,
  pub mask: Vec<bool>,
  pub mask_color: Color,
}

impl Layer {
  pub fn new(width: usize, height: usize) -> Self {
    Self {
      width,
      height,
      pixels: vec![0u8; width * height * 3],
      mask: vec![false; width * height],
      mask_color: Color::black(),
    }
  }

  pub fn set_pixel_color(&mut self, x: usize, y: usize, color: Color) {
    assert!(x < self.width && y < self.height);
    let idx = (y * self.width + x) * 3;
    self.pixels[idx] = (255.0 * color.r) as u8;
    self.pixels[idx + 1] = (255.0 * color.g) as u8;
    self.pixels[idx + 2] = (255.0 * color.b) as u8;

    let mask = color != self.mask_color;
    self.mask[y * self.width + x] = mask;
  }

  pub fn set_pixel(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8) {
    if x < self.width && y < self.height {
      let idx = (y * self.width + x) * 3;
      self.pixels[idx] = r;
      self.pixels[idx + 1] = g;
      self.pixels[idx + 2] = b;

      let mask = Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        w: None,
      } != self.mask_color;
      self.mask[y * self.width + x] = mask;
    }
  }

  /// Blits an entire image from disk into the layer at the specified offset
  pub fn overlay_path(&mut self, path: &'static str, x_offset: isize, y_offset: isize) {
    let img = image::open(path).expect("Failed to load image");
    self.overlay_image(&img, x_offset, y_offset);
  }

  /// Blits an image into the layer at the specified offset, using alpha for masking
  pub fn overlay_image(&mut self, img: &image::DynamicImage, x_offset: isize, y_offset: isize) {
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let bytes = rgba.as_raw();

    for y in 0..height as usize {
      for x in 0..width as usize {
        let idx = (y * width as usize + x) * 4;

        // pixel[3] is alpha
        let alpha = bytes[idx + 3];
        if alpha > 0 {
          self.set_pixel(
            (x as isize + x_offset) as usize,
            (y as isize + y_offset) as usize,
            bytes[idx],
            bytes[idx + 1],
            bytes[idx + 2],
          );
        }
      }
    }
  }
}
