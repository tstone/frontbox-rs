use std::path::Path;
use std::time::Duration;

use frontbox::animation::*;
use frontbox::prelude::*;
use image::Frame;

use crate::Gif;
use crate::{CanvasView, Layer};

#[derive(Clone)]
pub struct GifLayer {
  frames: Vec<Frame>,
  current_frame: usize,
  animation: Tween<Duration, usize>,
  active: bool,
}

impl GifLayer {
  pub fn new(path: impl AsRef<Path>, length: Duration, cycle: Cycle) -> Self {
    let frames = Gif::decode_from_path(path);
    Self {
      animation: Tween::new(length, Curve::Linear, vec![0, frames.len()], cycle),
      frames,
      current_frame: 0,
      active: true,
    }
  }

  pub fn current_frame_mut(&mut self) -> &mut usize {
    &mut self.current_frame
  }
}

impl Layer for GifLayer {
  fn render<'a>(&self, canvas: &mut CanvasView<'a>) {
    if let Some(frame) = self.frames.get(self.current_frame) {
      let image = frame.buffer();
      for x in 0..image.width() {
        for y in 0..image.height() {
          canvas.put_pixel(x, y, *image.get_pixel(x, y));
        }
      }
    }
  }
}

impl Accumulator<Duration> for GifLayer {
  fn accumulate(&mut self, delta: Duration) -> AccumulationResult<Duration> {
    if self.active {
      self.animation.accumulate(delta)
    } else {
      AccumulationResult::default()
    }
  }

  fn force(&mut self, current: Duration) {
    self.animation.force(current);
  }

  fn is_complete(&self) -> bool {
    self.animation.is_complete()
  }

  fn reset(&mut self) {
    self.animation.reset();
  }
}

impl Animation<Duration, usize> for GifLayer {
  fn sample(&self) -> usize {
    self.animation.sample()
  }

  fn play(&mut self) {
    self.active = true;
  }

  fn stop(&mut self) {
    self.active = false;
  }

  fn active(&self) -> bool {
    self.active
  }
}

impl Modulation<Duration, usize> for GifLayer {
  fn apply(&mut self, delta: Duration, target: &mut usize) {
    self.accumulate(delta);
    self.current_frame = *target;
  }
}
