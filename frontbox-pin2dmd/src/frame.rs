use crate::Renderable;

pub struct Frame {
  pub width: usize,
  pub height: usize,
  layers: Vec<Box<dyn Renderable>>,
  max_index: usize,
}

impl Frame {
  pub fn new(width: usize, height: usize) -> Self {
    Self {
      width,
      height,
      layers: Vec::new(),
      max_index: (width * height * 3) - 1,
    }
  }

  pub fn add(&mut self, img: impl Renderable + 'static) {
    self.layers.push(Box::new(img));
  }

  /// Flatten out frame into pixels for sending to the DMD
  pub fn to_pixels(&self) -> Vec<u8> {
    let mut pixels = vec![0u8; self.width * self.height * 3];

    for layer in &self.layers {
      let rendered = layer.render();
      for y in 0..rendered.image.height() as isize {
        for x in 0..rendered.image.width() as isize {
          let idx = ((y + rendered.offset_y) * self.width as isize + (x + rendered.offset_x)) * 3;

          // ignore out of bounds pixels
          if idx < 0 || (idx as usize) > self.max_index {
            continue;
          }

          let img = rendered.image.to_rgba8();
          let pixel = img.get_pixel(x as u32, y as u32);

          // ignore transparent pixels (alpha channel)
          if pixel[3] == 0 {
            continue;
          }

          pixels[idx as usize] = pixel[0];
          pixels[idx as usize + 1] = pixel[1];
          pixels[idx as usize + 2] = pixel[2];
        }
      }
    }

    pixels
  }
}
