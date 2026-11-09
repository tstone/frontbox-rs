use glam::Vec2;
use image::{Rgba, RgbaImage};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

static NONE: LazyLock<Images> = LazyLock::new(Images::default);

/// Decoded images by name, passed to sampling through [`FrameContext::images`](crate::scene::FrameContext::images)
///
/// Scenes refer to images by name only. [`Content::images_for`](crate::scene::Content::images_for) builds this for a
/// scene. Clones share pixel data.
#[derive(Debug, Clone, Default)]
pub struct Images(HashMap<String, Arc<Picture>>);

impl Images {
  pub fn new() -> Self {
    Self::default()
  }

  /// An empty set, for scenes with no images
  pub fn none() -> &'static Self {
    &NONE
  }

  pub fn insert(&mut self, name: impl Into<String>, picture: impl Into<Arc<Picture>>) {
    self.0.insert(name.into(), picture.into());
  }

  pub fn get(&self, name: &str) -> Option<&Picture> {
    self.0.get(name).map(Arc::as_ref)
  }
}

/// A decoded image, possibly animated, ready to sample
///
/// Each frame is stored as a mipmap chain, each level half the size of the one before. A sample that covers many pixels,
/// like a playfield LED, reads from the level whose pixels best match its size, so the image doesn't shimmer as it
/// moves. A sample the size of a pixel or smaller, like on a DMD, reads the exact pixel. All frames stay decoded.
#[derive(Debug)]
pub struct Picture {
  /// Mipmap levels for each frame, full size first
  frames: Vec<Vec<RgbaImage>>,
  /// Total playback time in seconds, from the file
  duration: f32,
}

impl Picture {
  pub fn new(image: RgbaImage) -> Self {
    Self::animated([image], 0.0)
  }

  /// Frames of equal size, and their total duration in seconds
  pub fn animated(frames: impl IntoIterator<Item = RgbaImage>, duration: f32) -> Self {
    Self { frames: frames.into_iter().map(levels).collect(), duration }
  }

  pub fn width(&self) -> u32 {
    self.frames.first().map_or(0, |levels| levels[0].width())
  }

  pub fn height(&self) -> u32 {
    self.frames.first().map_or(0, |levels| levels[0].height())
  }

  /// FrameContext count, 1 for a still image
  pub fn frames(&self) -> usize {
    self.frames.len()
  }

  /// Total duration in seconds (frames may not be evenly timed). 0 for a still image.
  pub fn duration(&self) -> f32 {
    self.duration
  }

  /// Memory used by pixel data, including every mipmap level
  pub(crate) fn bytes(&self) -> u64 {
    self.frames.iter().flatten().map(|image| image.as_raw().len() as u64).sum()
  }

  /// The color of a frame at a point in the unit box, with the image's top left at `-0.5`. `frame` is truncated and
  /// clamped to the frame count. `footprint` is the sample's radius on each axis in unit box units, and picks the mipmap
  /// level. Outside the box is transparent, except that a sample still overlapping it gets the nearest edge pixel, so an
  /// anti-aliased shape edge has a color to fade.
  pub(crate) fn color(&self, frame: f32, p: Vec2, footprint: Vec2) -> Rgba<u8> {
    let at = p + 0.5;
    let reaches = (at.cmpge(-footprint) & at.cmple(Vec2::ONE + footprint)).all();
    if !reaches || self.width() == 0 || self.height() == 0 {
      return Rgba([0, 0, 0, 0]);
    }
    let at = at.clamp(Vec2::ZERO, Vec2::ONE);
    let across = footprint * 2.0 * Vec2::new(self.width() as f32, self.height() as f32);
    let level = across.max_element().max(1.0).log2().round() as usize;
    let levels = &self.frames[(frame as usize).min(self.frames.len() - 1)];
    let image = &levels[level.min(levels.len() - 1)];

    let index = |at: f32, size: u32| ((at * size as f32) as u32).min(size - 1);
    *image.get_pixel(index(at.x, image.width()), index(at.y, image.height()))
  }
}

/// The mipmap chain for an image, down to 1 pixel
fn levels(image: RgbaImage) -> Vec<RgbaImage> {
  let mut levels = vec![image];
  while let Some(smaller) = levels.last().and_then(halved) {
    levels.push(smaller);
  }
  levels
}

/// Half the size, averaging each 2x2 block. `None` once it's 1 pixel.
fn halved(image: &RgbaImage) -> Option<RgbaImage> {
  let (width, height) = image.dimensions();
  if width.max(height) <= 1 || width == 0 || height == 0 {
    return None;
  }
  Some(RgbaImage::from_fn(width.div_ceil(2), height.div_ceil(2), |x, y| {
    let (left, top) = (x * 2, y * 2);
    let corners = [(left, top), (left + 1, top), (left, top + 1), (left + 1, top + 1)];
    average(corners.map(|(x, y)| *image.get_pixel(x.min(width - 1), y.min(height - 1))))
  }))
}

/// Weighted by alpha, so a transparent pixel's RGB doesn't bleed into the color
fn average(pixels: [Rgba<u8>; 4]) -> Rgba<u8> {
  let alpha: u32 = pixels.iter().map(|pixel| pixel[3] as u32).sum();
  if alpha == 0 {
    return Rgba([0, 0, 0, 0]);
  }
  let channel = |c: usize| {
    let total: u32 = pixels.iter().map(|pixel| pixel[c] as u32 * pixel[3] as u32).sum();
    ((total + alpha / 2) / alpha) as u8
  };
  Rgba([channel(0), channel(1), channel(2), ((alpha + 2) / 4) as u8])
}

#[cfg(test)]
mod tests {
  use super::*;

  const RED: Rgba<u8> = Rgba([255, 0, 0, 255]);
  const GREEN: Rgba<u8> = Rgba([0, 255, 0, 255]);
  const BLUE: Rgba<u8> = Rgba([0, 0, 255, 255]);
  const CLEAR: Rgba<u8> = Rgba([0, 255, 0, 0]);

  /// Red on the left half and blue on the right
  fn halves() -> Picture {
    Picture::new(RgbaImage::from_fn(4, 4, |x, _| if x < 2 { RED } else { BLUE }))
  }

  /// Every other pixel red and blue
  fn checkers(size: u32) -> Picture {
    Picture::new(RgbaImage::from_fn(size, size, |x, y| if (x + y) % 2 == 0 { RED } else { BLUE }))
  }

  #[test]
  fn a_sample_the_size_of_a_pixel_gets_exactly_that_pixel() {
    let picture = checkers(4);
    let pixel = |x: f32, y: f32| picture.color(0.0, Vec2::new(x, y), Vec2::splat(0.125));
    assert_eq!(pixel(-0.375, -0.375), RED);
    assert_eq!(pixel(-0.125, -0.375), BLUE);
    assert_eq!(pixel(-0.375, -0.125), BLUE);
    assert_eq!(pixel(0.375, 0.375), RED);
  }

  #[test]
  fn a_sample_smaller_than_a_pixel_gets_its_pixel() {
    assert_eq!(halves().color(0.0, Vec2::new(-0.01, 0.0), Vec2::ZERO), RED);
    assert_eq!(halves().color(0.0, Vec2::new(0.01, 0.0), Vec2::ZERO), BLUE);
  }

  #[test]
  fn the_edges_of_the_box_are_in_the_image_and_past_them_is_not() {
    let picture = halves();
    assert_eq!(picture.color(0.0, Vec2::new(-0.5, -0.5), Vec2::ZERO), RED);
    assert_eq!(picture.color(0.0, Vec2::new(0.5, 0.5), Vec2::ZERO), BLUE);
    assert_eq!(picture.color(0.0, Vec2::new(0.51, 0.0), Vec2::ZERO), Rgba([0, 0, 0, 0]));
    assert_eq!(picture.color(0.0, Vec2::new(0.0, -0.51), Vec2::ZERO), Rgba([0, 0, 0, 0]));
  }

  #[test]
  fn a_sample_just_past_an_edge_that_still_reaches_the_image_gets_the_nearest_pixel() {
    let picture = halves();
    let reach = Vec2::splat(0.1);
    assert_eq!(picture.color(0.0, Vec2::new(-0.55, 0.0), reach), RED);
    assert_eq!(picture.color(0.0, Vec2::new(0.55, 0.0), reach), BLUE);
    assert_eq!(picture.color(0.0, Vec2::new(-0.65, 0.0), reach), Rgba([0, 0, 0, 0]));
    assert_eq!(picture.color(0.0, Vec2::new(f32::NAN, 0.0), reach), Rgba([0, 0, 0, 0]));
  }

  #[test]
  fn a_sample_covering_many_pixels_gets_their_average() {
    let color = checkers(64).color(0.0, Vec2::ZERO, Vec2::splat(0.1));
    assert!((126..=129).contains(&color[0]) && (126..=129).contains(&color[2]), "{color:?}");
  }

  #[test]
  fn a_sample_stays_steady_as_fine_detail_moves_under_it() {
    let picture = checkers(256);
    let colors: Vec<_> = (0..50).map(|i| picture.color(0.0, Vec2::new(i as f32 / 300.0, 0.0), Vec2::splat(0.1))).collect();
    let reds = || colors.iter().map(|color| color[0]);
    assert!(reds().max().unwrap() - reds().min().unwrap() <= 2, "{colors:?}");
  }

  #[test]
  fn a_flat_image_still_shrinks_along_its_long_side() {
    let picture = Picture::new(RgbaImage::from_fn(8, 1, |x, _| if x % 2 == 0 { RED } else { BLUE }));
    assert_eq!(picture.frames[0][1].dimensions(), (4, 1));
    assert_eq!(picture.frames[0].last().unwrap().dimensions(), (1, 1));
  }

  #[test]
  fn what_is_under_a_transparent_pixel_stays_out_of_the_average() {
    assert_eq!(average([RED, CLEAR, RED, CLEAR]), Rgba([255, 0, 0, 128]));
  }

  #[test]
  fn an_image_with_no_pixels_draws_nothing() {
    let picture = Picture::new(RgbaImage::new(0, 0));
    assert_eq!(picture.color(0.0, Vec2::ZERO, Vec2::ZERO), Rgba([0, 0, 0, 0]));
  }

  /// One frame each of red, green, and blue
  fn three_frames() -> Picture {
    let frames = [RED, GREEN, BLUE].map(|color| RgbaImage::from_pixel(2, 2, color));
    Picture::animated(frames, 0.3)
  }

  #[test]
  fn each_frame_is_drawn_by_its_number() {
    let picture = three_frames();
    let colors = [0.0, 1.0, 2.0].map(|frame| picture.color(frame, Vec2::ZERO, Vec2::ZERO));
    assert_eq!(colors, [RED, GREEN, BLUE]);
    assert_eq!(picture.frames(), 3);
    assert_eq!(picture.duration(), 0.3);
  }

  #[test]
  fn a_fraction_of_a_frame_is_dropped() {
    assert_eq!(three_frames().color(0.99, Vec2::ZERO, Vec2::ZERO), RED);
    assert_eq!(three_frames().color(1.99, Vec2::ZERO, Vec2::ZERO), GREEN);
  }

  #[test]
  fn frames_past_the_ends_are_the_ends() {
    assert_eq!(three_frames().color(-4.0, Vec2::ZERO, Vec2::ZERO), RED);
    assert_eq!(three_frames().color(3.0, Vec2::ZERO, Vec2::ZERO), BLUE);
    assert_eq!(three_frames().color(f32::INFINITY, Vec2::ZERO, Vec2::ZERO), BLUE);
    assert_eq!(three_frames().color(f32::NAN, Vec2::ZERO, Vec2::ZERO), RED);
  }

  #[test]
  fn an_image_that_is_not_animated_has_one_frame_whatever_is_asked_for() {
    assert_eq!(halves().frames(), 1);
    assert_eq!(halves().duration(), 0.0);
    assert_eq!(halves().color(5.0, Vec2::new(-0.25, 0.0), Vec2::ZERO), RED);
  }

  #[test]
  fn each_frame_is_averaged_on_its_own() {
    let color = three_frames().color(1.0, Vec2::ZERO, Vec2::splat(1.0));
    assert_eq!(color, GREEN);
  }

  #[test]
  fn no_frames_draw_nothing() {
    let picture = Picture::animated([], 0.0);
    assert_eq!(picture.frames(), 0);
    assert_eq!(picture.color(0.0, Vec2::ZERO, Vec2::ZERO), Rgba([0, 0, 0, 0]));
  }
}
