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

  fn axis_length(width: u32, height: u32, angle_degrees: f32) -> f32 {
    let theta = angle_degrees.to_radians();
    width as f32 * theta.cos().abs() + height as f32 * theta.sin().abs()
  }

  fn gradient_axis_position(x: u32, y: u32, width: u32, height: u32, angle_degrees: f32) -> f32 {
    let theta = angle_degrees.to_radians();
    let (dx, dy) = (theta.cos(), theta.sin());

    let cx = x as f32 - (width as f32 - 1.0) / 2.0;
    let cy = y as f32 - (height as f32 - 1.0) / 2.0;
    let projection = cx * dx + cy * dy;

    // Shift from centered [-LEN/2, LEN/2] into [0, LEN]
    projection + Self::axis_length(width, height, angle_degrees) / 2.0
  }

  fn sample_gradient_stops(stops: &[GradientStop], axis_pos: f32, axis_len: u16) -> Rgba<u8> {
    if stops.is_empty() {
      return Rgba([0, 0, 0, 0]);
    }
    if stops.len() == 1 {
      return stops[0].color;
    }

    let axis_pos = axis_pos.clamp(0.0, axis_len as f32);

    let mut lower = &stops[0];
    let mut upper = &stops[stops.len() - 1];

    for pair in stops.windows(2) {
      let lo_pos = pair[0].position.to_absolute(axis_len) as f32;
      let hi_pos = pair[1].position.to_absolute(axis_len) as f32;
      if axis_pos >= lo_pos && axis_pos <= hi_pos {
        lower = &pair[0];
        upper = &pair[1];
        break;
      }
    }

    let lo_pos = lower.position.to_absolute(axis_len) as f32;
    let hi_pos = upper.position.to_absolute(axis_len) as f32;

    if hi_pos <= lo_pos {
      return lower.color;
    }

    let local_t = (axis_pos - lo_pos) / (hi_pos - lo_pos);
    lower.color.mix_with(upper.color, local_t)
  }
}

impl LayerGenerator for Rectangle {
  fn generate(&self, viewport: &Size<u32>) -> Layer {
    let width = self.size.width.to_absolute(viewport.width);
    let height = self.size.height.to_absolute(viewport.height);
    let mut buffer = RgbaImage::new(width, height);

    for y in 0..height {
      for x in 0..width {
        let pixel = match &self.fill {
          Fill2d::Transparent => Rgba([0, 0, 0, 0]),
          Fill2d::Solid(color) => *color,
          Fill2d::Gradient(stops, angle) => {
            let axis_len = Self::axis_length(width, height, *angle).round() as u16;
            let axis_pos = Self::gradient_axis_position(x, y, width, height, *angle);
            Self::sample_gradient_stops(stops, axis_pos, axis_len)
          }
        };

        if pixel[3] == 0 {
          continue;
        }

        buffer.put_pixel(x, y, pixel);
      }
    }

    Layer {
      img: DynamicImage::ImageRgba8(buffer),
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
