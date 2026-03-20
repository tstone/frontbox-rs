use crate::{Pin2Dmd, Renderable};

pub struct Frame {
  pub width: usize,
  pub height: usize,
  layers: Vec<Box<dyn Renderable>>,
}

impl Frame {
  pub fn new(width: usize, height: usize) -> Self {
    Self {
      width,
      height,
      layers: Vec::new(),
    }
  }

  pub fn for_dmd(dmd: &Pin2Dmd) -> Self {
    Self::new(dmd.width(), dmd.height())
  }

  pub fn add(&mut self, img: impl Renderable + 'static) {
    self.layers.push(Box::new(img));
  }

  /// Flatten out frame into pixels for sending to the DMD
  pub fn render(&mut self) -> Vec<u8> {
    let mut pixels = vec![0u8; self.width * self.height * 3];

    for layer in &mut self.layers {
      let rendered = layer.render();
      let img = rendered.image.to_rgba8();

      for y in 0..img.height() as isize {
        for x in 0..img.width() as isize {
          let dest_x = x + rendered.offset_x;
          let dest_y = y + rendered.offset_y;

          // ignore out of bounds pixels
          if dest_x < 0
            || dest_y < 0
            || dest_x >= self.width as isize
            || dest_y >= self.height as isize
          {
            continue;
          }

          let pixel = img.get_pixel(x as u32, y as u32);

          // ignore transparent pixels
          if pixel[3] == 0 {
            continue;
          }

          let idx = (dest_y as usize * self.width + dest_x as usize) * 3;
          pixels[idx] = pixel[0];
          pixels[idx + 1] = pixel[1];
          pixels[idx + 2] = pixel[2];
        }
      }
    }

    pixels
  }
}
