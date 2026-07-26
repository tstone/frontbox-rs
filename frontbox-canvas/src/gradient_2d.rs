use frontbox::prelude::color_sequence::GradientStop;
use frontbox::prelude::*;

use crate::Size;

#[derive(Debug, Default, Clone)]
pub struct Gradient2d {
  stops: Vec<GradientStop>,
  theta: f32, // radians
  axis_length: f32,
  bounds: Size<u32>,
}

impl Gradient2d {
  pub fn new(stops: Vec<GradientStop>, angle: f32, bounds: Size<u32>) -> Self {
    let theta = angle.to_radians();
    let axis_length =
      bounds.width as f32 * theta.cos().abs() + bounds.height as f32 * theta.sin().abs();

    Gradient2d {
      stops,
      theta,
      axis_length,
      bounds,
    }
  }

  // calculate where on the axis a given pixel position within the bounds lies
  fn axis_position(&self, x: u32, y: u32) -> f32 {
    let (dx, dy) = (self.theta.cos(), self.theta.sin());

    let cx = x as f32 - (self.bounds.width as f32 - 1.0) / 2.0;
    let cy = y as f32 - (self.bounds.height as f32 - 1.0) / 2.0;
    let projection = cx * dx + cy * dy;

    // Shift from centered [-LEN/2, LEN/2] into [0, LEN]
    projection + self.axis_length / 2.0
  }

  pub fn sample_at_index(&self, i: usize, width: u32) -> Rgba<u8> {
    let x = i as u32 % width;
    let y = i as u32 / width;
    self.sample_at_point(x, y)
  }

  pub fn sample_at_point(&self, x: u32, y: u32) -> Rgba<u8> {
    if self.stops.is_empty() {
      return Rgba([0, 0, 0, 0]);
    }
    if self.stops.len() == 1 {
      return self.stops[0].color;
    }

    let axis_pos = self.axis_position(x, y).clamp(0.0, self.axis_length);
    let mut lower = &self.stops[0];
    let mut upper = &self.stops[self.stops.len() - 1];

    for pair in self.stops.windows(2) {
      let lo_pos = pair[0].position.to_absolute(self.axis_length as u16) as f32;
      let hi_pos = pair[1].position.to_absolute(self.axis_length as u16) as f32;
      if axis_pos >= lo_pos && axis_pos <= hi_pos {
        lower = &pair[0];
        upper = &pair[1];
        break;
      }
    }

    let lo_pos = lower.position.to_absolute(self.axis_length as u16) as f32;
    let hi_pos = upper.position.to_absolute(self.axis_length as u16) as f32;

    if hi_pos <= lo_pos {
      return lower.color;
    }

    let local_t = (axis_pos - lo_pos) / (hi_pos - lo_pos);
    lower.color.mix_with(upper.color, local_t)
  }
}
