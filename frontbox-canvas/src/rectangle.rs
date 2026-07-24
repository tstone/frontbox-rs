use frontbox::prelude::color_sequence::GradientStop;
use frontbox::prelude::*;
use image::{DynamicImage, Rgba, RgbaImage};

use crate::*;

pub struct Rectangle {
  size: Size<Extent<u32>>,
  pub horizontal: Horizontal,
  pub vertical: Vertical,
  pub fill: Fill2d,
  pub border: Option<Border>,
}

impl Rectangle {
  pub fn new(width: impl Into<Extent<u32>>, height: impl Into<Extent<u32>>, fill: Fill2d) -> Self {
    Self {
      size: Size::new(width.into(), height.into()),
      horizontal: Horizontal::default(),
      vertical: Vertical::default(),
      fill,
      border: None,
    }
  }

  pub fn transparent(width: impl Into<Extent<u32>>, height: impl Into<Extent<u32>>) -> Self {
    Self {
      size: Size::new(width.into(), height.into()),
      horizontal: Horizontal::default(),
      vertical: Vertical::default(),
      fill: Fill2d::Transparent,
      border: None,
    }
  }

  pub fn solid(
    width: impl Into<Extent<u32>>,
    height: impl Into<Extent<u32>>,
    color: Rgba<u8>,
  ) -> Self {
    Self {
      size: Size::new(width.into(), height.into()),
      horizontal: Horizontal::default(),
      vertical: Vertical::default(),
      fill: Fill2d::Solid(color),
      border: None,
    }
  }

  pub fn fade(
    width: impl Into<Extent<u32>>,
    height: impl Into<Extent<u32>>,
    from: Rgba<u8>,
    to: Rgba<u8>,
    angle: f32,
  ) -> Self {
    Self {
      size: Size::new(width.into(), height.into()),
      horizontal: Horizontal::default(),
      vertical: Vertical::default(),
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

  pub fn gradient(
    width: impl Into<Extent<u32>>,
    height: impl Into<Extent<u32>>,
    stops: Vec<GradientStop>,
    angle: f32,
  ) -> Self {
    Self {
      size: Size::new(width.into(), height.into()),
      horizontal: Horizontal::default(),
      vertical: Vertical::default(),
      fill: Fill2d::Gradient(stops, angle),
      border: None,
    }
  }

  pub fn with_horizontal(mut self, pos: Horizontal) -> Self {
    self.horizontal = pos;
    self
  }

  pub fn with_vertical(mut self, pos: Vertical) -> Self {
    self.vertical = pos;
    self
  }

  pub fn with_border(mut self, width: u8, color: Rgba<u8>) -> Self {
    self.border = Some(Border::new(width, color));
    self
  }
}

impl LayerGenerator for Rectangle {
  fn generate(&self, viewport: &Size<u32>) -> Layer {
    let width = self.size.width.to_absolute(viewport.width);
    let height = self.size.height.to_absolute(viewport.height);
    let mut buffer = RgbaImage::new(width, height);
    let gradient = if let Fill2d::Gradient(stops, angle) = &self.fill {
      Gradient2d::new(stops.clone(), *angle, Size::new(width, height))
    } else {
      Gradient2d::default()
    };

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

        buffer.put_pixel(x, y, pixel);
      }
    }

    Layer {
      image: DynamicImage::ImageRgba8(buffer),
      horizontal: self.horizontal,
      vertical: self.vertical,
    }
  }
}

#[derive(Debug, Clone)]
pub enum Fill2d {
  Transparent,
  Solid(Rgba<u8>),
  Gradient(Vec<GradientStop>, f32),
}
