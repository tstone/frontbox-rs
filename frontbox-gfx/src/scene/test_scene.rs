use std::ops::{Deref, DerefMut};

use glam::Vec2;
use image::Rgba;

use crate::scene::{Frame, GfxComposition, Layer, Sample, ShapePose, Timing, Viewport};

/// A composition and what it's seen on, as LED and DMD scenes hold one, for tests to sample without saying which each time
#[derive(Debug, Clone)]
pub(crate) struct Scene {
  pub viewport: Viewport,
  pub composition: GfxComposition,
}

impl Deref for Scene {
  type Target = GfxComposition;

  fn deref(&self) -> &GfxComposition {
    &self.composition
  }
}

impl DerefMut for Scene {
  fn deref_mut(&mut self) -> &mut GfxComposition {
    &mut self.composition
  }
}

impl Scene {
  pub fn new(viewport: Viewport, layers: Vec<Layer>) -> Self {
    Self::with_composition(viewport, GfxComposition::new(layers))
  }

  pub fn with_composition(viewport: Viewport, composition: GfxComposition) -> Self {
    Self { viewport, composition }
  }

  pub fn timing(self, timing: Timing) -> Self {
    Self { composition: self.composition.timing(timing), ..self }
  }

  pub fn middle_cycle(&self) -> Self {
    Self { composition: self.composition.middle_cycle(), ..self.clone() }
  }

  fn seen<'a>(&self, frame: &Frame<'a>) -> Frame<'a> {
    Frame { viewport: self.viewport, ..*frame }
  }

  pub fn sample(&self, at: &Sample, frame: &Frame) -> Rgba<u8> {
    self.composition.sample(at, &self.seen(frame))
  }

  pub fn outlines(&self, frame: &Frame) -> Vec<Vec<Vec2>> {
    self.composition.outlines(&self.seen(frame))
  }

  pub fn mask_outlines(&self, frame: &Frame) -> Vec<Vec<Vec2>> {
    self.composition.mask_outlines(&self.seen(frame))
  }

  pub fn mask_poses(&self, frame: &Frame) -> Vec<Option<ShapePose>> {
    self.composition.mask_poses(&self.seen(frame))
  }
}
