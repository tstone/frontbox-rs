use crate::prelude::*;

pub struct Rotate {
  pub direction: Rotation,
  pub degrees: f32,
  pub other: Box<dyn ColorSequence>,
}

impl ColorSequence for Rotate {
  fn render(&self, count: usize) -> Vec<Rgba<u8>> {
    let mut colors = self.other.render(count);
    let steps = (self.degrees / 360.0 * count as f32).round() as usize % count;

    match self.direction {
      Rotation::Clockwise => colors.rotate_right(steps),
      Rotation::CounterClockwise => colors.rotate_left(steps),
    }

    colors
  }
}

pub enum Rotation {
  Clockwise,
  CounterClockwise,
}
