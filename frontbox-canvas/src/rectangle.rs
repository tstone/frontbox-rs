use frontbox::prelude::color_sequence::GradientStop;
use frontbox::prelude::*;
use image::Rgba;

use crate::*;

pub struct Rectangle {
  pub fill: Fill2d,
  pub border: Option<Border>,
}

impl Rectangle {
  pub fn new(fill: Fill2d) -> Self {
    Self { fill, border: None }
  }

  pub fn transparent() -> Self {
    Self {
      fill: Fill2d::Transparent,
      border: None,
    }
  }

  pub fn solid(color: Rgba<u8>) -> Self {
    Self {
      fill: Fill2d::Solid(color),
      border: None,
    }
  }

  pub fn fade(from: Rgba<u8>, to: Rgba<u8>, angle: f32) -> Self {
    Self {
      fill: Fill2d::Gradient(
        vec![
          GradientStop::new(Extent::zero(), from),
          GradientStop::new(Extent::full(), to),
        ],
        angle,
      ),
      border: None,
    }
  }

  pub fn gradient(stops: Vec<GradientStop>, angle: f32) -> Self {
    Self {
      fill: Fill2d::Gradient(stops, angle),
      border: None,
    }
  }

  pub fn with_border(mut self, width: u8, color: Rgba<u8>) -> Self {
    self.border = Some(Border::new(width, color));
    self
  }
}

impl Layer for Rectangle {
  fn render<'a>(&self, canvas: &mut CanvasView<'a>) {
    let width = canvas.bounds.width;
    let height = canvas.bounds.height;

    let gradient = if let Fill2d::Gradient(stops, angle) = &self.fill {
      Gradient2d::new(stops.clone(), *angle, Size::new(width, height))
    } else {
      Gradient2d::default()
    };

    // TODO: render border

    for y in 0..height {
      for x in 0..width {
        let pixel = match &self.fill {
          Fill2d::Transparent => Rgba([0, 0, 0, 0]),
          Fill2d::Solid(color) => *color,
          Fill2d::Gradient(..) => gradient.sample_at_point(x, y),
        };

        if pixel[3] == 0 {
          continue;
        }

        canvas.buffer.put_pixel_at(x, y, pixel);
      }
    }
  }
}

#[derive(Debug, Clone)]
pub enum Fill2d {
  Transparent,
  Solid(Rgba<u8>),
  Gradient(Vec<GradientStop>, f32),
}

#[derive(Debug, Clone, Copy)]
pub struct Border {
  pub color: Rgba<u8>,
  pub width: u8,
}

impl Border {
  pub fn new(width: u8, color: Rgba<u8>) -> Self {
    Self { color, width }
  }
}
