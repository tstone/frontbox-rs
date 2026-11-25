use std::ops::{Deref, DerefMut};

use glam::Vec2;
use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::scene::{
  Graph, Layer, LayerFrame, Modulators, Regions, Sample, SceneError, Shape, Surface, Timing,
  Variables, Visits, check_sweeps,
};

/// Layers sharing variables and timing, as a scene file holds them but with nothing about hardware
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct Composition {
  #[serde(default, skip_serializing_if = "Variables::is_empty")]
  pub variables: Variables,
  #[serde(default, skip_serializing_if = "Modulators::is_empty")]
  pub modulators: Modulators,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub timing: Option<Timing>,
  #[serde(default)]
  pub layers: Vec<Layer>,
}

impl Composition {
  pub fn check(&self) -> Result<(), SceneError> {
    if let Some(timing) = self.timing {
      timing.check()?;
    }
    for (name, modulator) in &self.modulators {
      modulator.check(name)?;
    }
    self
      .layers
      .iter()
      .enumerate()
      .try_for_each(|(index, layer)| layer.check(index, self.timing))?;
    let layers: Vec<_> = self
      .layers
      .iter()
      .enumerate()
      .map(|(index, layer)| (index, layer, ()))
      .collect();
    check_sweeps(&layers)
  }

  pub fn image_names(&self) -> impl Iterator<Item = &str> {
    self.layers.iter().flat_map(Layer::image_names)
  }

  pub fn is_complete(&self, time: f32) -> bool {
    self.timing.is_some_and(|timing| timing.is_complete(time))
  }

  pub fn cycle_time(&self, cycle: u32, time: f32) -> f32 {
    self
      .timing
      .map_or(time, |timing| timing.cycle_time(cycle, time))
  }

  pub fn middle_cycle_time(&self, time: f32) -> f32 {
    self.cycle_time(1, time)
  }
}

impl Regions for Composition {
  fn region(&self, layer: &str) -> Option<&Shape> {
    self
      .layers
      .iter()
      .find(|other| other.name.as_deref() == Some(layer))?
      .region
      .as_ref()
  }
}

/// A composition with every layer on one surface, so tests don't pass it every time
#[derive(Debug, Clone)]
pub(crate) struct Scene {
  pub surface: Surface,
  pub composition: Composition,
}

impl Deref for Scene {
  type Target = Composition;

  fn deref(&self) -> &Composition {
    &self.composition
  }
}

impl DerefMut for Scene {
  fn deref_mut(&mut self) -> &mut Composition {
    &mut self.composition
  }
}

impl Scene {
  pub fn new(surface: Surface, layers: Vec<Layer>) -> Self {
    Self::with_composition(
      surface,
      Composition {
        layers,
        ..Default::default()
      },
    )
  }

  pub fn with_composition(surface: Surface, composition: Composition) -> Self {
    Self {
      surface,
      composition,
    }
  }

  pub fn timing(mut self, timing: Timing) -> Self {
    self.composition.timing = Some(timing);
    self
  }

  pub fn middle_cycle(&self) -> Self {
    let mut scene = self.clone();
    scene.composition.timing = self.timing.map(Timing::middle_cycle);
    scene
  }

  /// The frame as this scene sees it: on its surface, with its timing and defaults
  fn seen<'a>(&'a self, frame: &LayerFrame<'a>) -> LayerFrame<'a> {
    LayerFrame {
      surface: self.surface,
      timing: self.timing,
      defaults: &self.variables,
      modulators: &self.modulators,
      regions: &self.composition,
      ..*frame
    }
  }

  /// Every layer drawn bottom to top
  pub fn sample(&self, at: &Sample, frame: &LayerFrame) -> Rgba<u8> {
    let frame = self.seen(frame);
    self.layers.iter().fold(Rgba([0, 0, 0, 0]), |below, layer| {
      layer.prepare(&frame, at.radius).composite(at, below)
    })
  }

  /// Each layer's visits, for [`Self::sample_visited`]
  pub fn visits(&self, graph: &Graph, frame: &LayerFrame) -> Vec<Visits> {
    let frame = self.seen(frame);
    self
      .layers
      .iter()
      .map(|layer| layer.visits(graph, &frame))
      .collect()
  }

  /// [`Self::sample`] with each layer's visits
  pub fn sample_visited(&self, at: &Sample, frame: &LayerFrame, visits: &[Visits]) -> Rgba<u8> {
    let frame = self.seen(frame);
    self
      .layers
      .iter()
      .zip(visits)
      .fold(Rgba([0, 0, 0, 0]), |below, (layer, visits)| {
        let frame = LayerFrame {
          visits: Some(visits),
          ..frame
        };
        layer.prepare(&frame, at.radius).composite(at, below)
      })
  }

  pub fn outlines(&self, frame: &LayerFrame) -> Vec<Vec<Vec<Vec2>>> {
    let frame = self.seen(frame);
    self
      .layers
      .iter()
      .map(|layer| layer.outline(&frame))
      .collect()
  }
}
