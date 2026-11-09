use std::ops::{Deref, DerefMut};

use glam::Vec2;
use image::Rgba;

use crate::scene::{FrameContext, GfxComposition, Layer, Sample, ShapePose, Timing, Surface};

/// A composition paired with a viewport, like `LedScene` and `DmdScene`, so tests don't pass the viewport every time
#[derive(Debug, Clone)]
pub(crate) struct Scene {
  pub surface: Surface,
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
  pub fn new(surface: Surface, layers: Vec<Layer>) -> Self {
    Self::with_composition(surface, GfxComposition::new(layers))
  }

  pub fn with_composition(surface: Surface, composition: GfxComposition) -> Self {
    Self { surface, composition }
  }

  pub fn timing(self, timing: Timing) -> Self {
    Self { composition: self.composition.timing(timing), ..self }
  }

  pub fn middle_cycle(&self) -> Self {
    Self { composition: self.composition.middle_cycle(), ..self.clone() }
  }

  fn seen<'a>(&self, frame: &FrameContext<'a>) -> FrameContext<'a> {
    FrameContext { surface: self.surface, ..*frame }
  }

  pub fn sample(&self, at: &Sample, frame: &FrameContext) -> Rgba<u8> {
    self.composition.sample(at, &self.seen(frame))
  }

  pub fn outlines(&self, frame: &FrameContext) -> Vec<Vec<Vec2>> {
    self.composition.outlines(&self.seen(frame))
  }

  pub fn mask_outlines(&self, frame: &FrameContext) -> Vec<Vec<Vec2>> {
    self.composition.mask_outlines(&self.seen(frame))
  }

  pub fn mask_poses(&self, frame: &FrameContext) -> Vec<Option<ShapePose>> {
    self.composition.mask_poses(&self.seen(frame))
  }
}
