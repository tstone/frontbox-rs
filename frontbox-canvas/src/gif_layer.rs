use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

use frontbox::animation::*;
use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, Frame};

use crate::{CanvasView, Layer};

#[derive(Clone)]
pub struct GifLayer {
  frames: Vec<Frame>,
  current_frame: usize,
  animation: Tween<Duration, usize>,
}

impl GifLayer {
  pub fn new(path: impl Into<&'static Path>, length: Duration, cycle: AnimationCycle) -> Self {
    let path = path.into();
    let file_in =
      BufReader::new(File::open(path).expect(format!("Failed to load gif at {:?}", path).as_str()));
    let decoder = GifDecoder::new(file_in).unwrap();
    let frames = decoder.into_frames();
    let frames = frames.collect_frames().expect("error decoding gif");
    Self {
      animation: Tween::new(length, Curve::Linear, vec![0, frames.len()], cycle),
      frames,
      current_frame: 0,
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
    self.animation.accumulate(delta)
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
}

impl Modulation<Duration, usize> for GifLayer {
  fn apply(&mut self, delta: Duration, target: &mut usize) {
    self.accumulate(delta);
    self.current_frame = *target;
  }
}
