use image::{DynamicImage, Rgba, RgbaImage};

use crate::{Pin2Dmd, Renderable, RenderableImage};

pub struct Frame {
  size: FrameSize,
  layers: Vec<Box<dyn Renderable>>,
  fill: Fill,
  border: Option<(Rgba<u8>, u8)>,
}

impl Frame {
  pub fn new(width: usize, height: usize, fill: Fill) -> Self {
    Self {
      size: FrameSize { width, height },
      layers: Vec::new(),
      fill,
      border: None,
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

  pub fn with_fill(mut self, fill: Fill) -> Self {
    self.fill = fill;
    self
  }

  pub fn with_border(mut self, color: Rgba<u8>, thickness: u8) -> Self {
    self.border = Some((color, thickness));
    self
  }

  pub fn for_dmd(dmd: &Pin2Dmd) -> Self {
    Self::new(dmd.width(), dmd.height(), Fill::Transparent)
  }

  pub fn add(&mut self, img: impl Renderable + 'static) {
    self.layers.push(Box::new(img));
  }
}

impl Renderable for Frame {
  fn render(&self, parent: &FrameSize) -> RenderableImage {
    let mut output = RgbaImage::new(self.width() as u32, self.height() as u32);

    match self.fill {
      Fill::Transparent => {}
      Fill::Solid(color) => {
        for y in 0..self.height() as u32 {
          for x in 0..self.width() as u32 {
            output.put_pixel(x, y, color);
          }
        }
      }
      Fill::VerticalGradient(top, bottom) => {
        for y in 0..self.height() as u32 {
          let ratio = y as f32 / self.height() as f32;
          let color = Rgba([
            (top[0] as f32 * (1.0 - ratio) + bottom[0] as f32 * ratio) as u8,
            (top[1] as f32 * (1.0 - ratio) + bottom[1] as f32 * ratio) as u8,
            (top[2] as f32 * (1.0 - ratio) + bottom[2] as f32 * ratio) as u8,
            (top[3] as f32 * (1.0 - ratio) + bottom[3] as f32 * ratio) as u8,
          ]);
          for x in 0..self.width() as u32 {
            output.put_pixel(x, y, color);
          }
        }
      }
      Fill::HorizontalGradient(left, right) => {
        for x in 0..self.width() as u32 {
          let ratio = x as f32 / self.width() as f32;
          let color = Rgba([
            (left[0] as f32 * (1.0 - ratio) + right[0] as f32 * ratio) as u8,
            (left[1] as f32 * (1.0 - ratio) + right[1] as f32 * ratio) as u8,
            (left[2] as f32 * (1.0 - ratio) + right[2] as f32 * ratio) as u8,
            (left[3] as f32 * (1.0 - ratio) + right[3] as f32 * ratio) as u8,
          ]);
          for y in 0..self.height() as u32 {
            output.put_pixel(x, y, color);
          }
        }
      }
    }

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

    // render border if present
    if let Some((color, thickness)) = self.border {
      for t in 0..thickness {
        // top and bottom borders
        for x in t as u32..(self.width() as u32 - t as u32) {
          output.put_pixel(x, t as u32, color);
          output.put_pixel(x, (self.height() as u32 - 1) - t as u32, color);
        }
        // left and right borders
        for y in t as u32..(self.height() as u32 - t as u32) {
          output.put_pixel(t as u32, y, color);
          output.put_pixel((self.width() as u32 - 1) - t as u32, y, color);
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

pub enum Fill {
  Transparent,
  Solid(Rgba<u8>),
  VerticalGradient(Rgba<u8>, Rgba<u8>),
  HorizontalGradient(Rgba<u8>, Rgba<u8>),
}
