use image::{DynamicImage, RgbaImage};

use crate::{Pin2Dmd, Renderable, RenderableImage};

pub struct Frame {
  size: FrameSize,
  layers: Vec<Box<dyn Renderable>>,
}

impl Frame {
  pub fn new(width: usize, height: usize) -> Self {
    Self {
      size: FrameSize { width, height },
      layers: Vec::new(),
    }
  }

  pub fn width(&self) -> usize {
    self.size.width
  }

  pub fn height(&self) -> usize {
    self.size.height
  }

  pub fn size(&self) -> &FrameSize {
    &self.size
  }

  pub fn for_dmd(dmd: &Pin2Dmd) -> Self {
    Self::new(dmd.width(), dmd.height())
  }

  pub fn add(&mut self, img: impl Renderable + 'static) {
    self.layers.push(Box::new(img));
  }
}

impl Renderable for Frame {
  fn render(&self, parent: &FrameSize) -> RenderableImage {
    let mut output = RgbaImage::new(self.width() as u32, self.height() as u32);

    for layer in &self.layers {
      let rendered = layer.render(parent);
      let img = rendered.image.to_rgba8();

      for y in 0..img.height() as isize {
        for x in 0..img.width() as isize {
          let dest_x = x + rendered.offset_x;
          let dest_y = y + rendered.offset_y;

          if dest_x < 0
            || dest_y < 0
            || dest_x >= self.width() as isize
            || dest_y >= self.height() as isize
          {
            continue;
          }

          let pixel = img.get_pixel(x as u32, y as u32);
          if pixel[3] == 0 {
            continue;
          }

          output.put_pixel(dest_x as u32, dest_y as u32, *pixel);
        }
      }
    }

    RenderableImage {
      image: DynamicImage::ImageRgba8(output),
      offset_x: 0,
      offset_y: 0,
    }
  }
}
pub struct FrameSize {
  pub width: usize,
  pub height: usize,
}

impl FrameSize {
  pub fn new(width: usize, height: usize) -> Self {
    Self { width, height }
  }

  pub fn for_dmd(dmd: &Pin2Dmd) -> Self {
    Self {
      width: dmd.width(),
      height: dmd.height(),
    }
  }
}

impl From<Pin2Dmd> for FrameSize {
  fn from(dmd: Pin2Dmd) -> Self {
    Self::for_dmd(&dmd)
  }
}
