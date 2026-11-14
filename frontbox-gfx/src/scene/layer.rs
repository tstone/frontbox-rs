use glam::{Affine2, Vec2};
use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::scene::{Bounds, Env, Fill, LayerFrame, Graph, GraphMask, Mask, Sample, SceneError, Shape, ShapePose, Speed, Surface, Timing, Transform, Visits, Prop};

/// A fill drawn in a region
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
  /// Display name for editors and error messages. Doesn't need to be unique.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub name: Option<String>,
  /// Skipped when drawing, but still validated by `check`
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
  pub hidden: bool,
  pub fill: Fill,
  /// Where the layer draws, and what its fill is stretched over. `None` covers the whole viewport.
  #[serde(default, alias = "shape", skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub region: Option<Shape>,
  /// `None` shows the whole layer
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub mask: Option<Mask>,
  /// Limits the layer to a graph's nodes, with traversals over it. `None` draws at every sample.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub graph: Option<GraphMask>,
}

impl Layer {
  pub fn new(fill: Fill) -> Self {
    Self { name: None, hidden: false, fill, region: None, mask: None, graph: None }
  }

  pub fn name(mut self, name: impl Into<String>) -> Self {
    self.name = Some(name.into());
    self
  }

  pub fn hidden(mut self) -> Self {
    self.hidden = true;
    self
  }

  pub fn region(mut self, region: Shape) -> Self {
    self.region = Some(region);
    self
  }

  pub fn mask(mut self, mask: Mask) -> Self {
    self.mask = Some(mask);
    self
  }

  pub fn graph(mut self, graph: GraphMask) -> Self {
    self.graph = Some(graph);
    self
  }

  /// How logs and errors refer to the layer at `index`: its name, or else its 1-based position from the bottom (as the
  /// console numbers them)
  pub fn label(&self, index: usize) -> String {
    match &self.name {
      Some(name) => format!("layer \"{name}\""),
      None => format!("layer {}", index + 1),
    }
  }

  pub(crate) fn is_animated(&self) -> bool {
    self.fill.is_animated()
      || self.region.as_ref().is_some_and(Shape::is_animated)
      || self.mask.as_ref().is_some_and(Mask::is_animated)
      || self.graph.as_ref().is_some_and(GraphMask::is_animated)
  }

  /// Names of the images the layer and its mask use
  pub fn image_names(&self) -> impl Iterator<Item = &str> {
    let masked = self.mask.as_ref().and_then(|mask| mask.fill.base.image_name());
    self.fill.base.image_name().into_iter().chain(masked)
  }

  /// Validates what the types can't enforce, for the layer at `index` in a scene with `timing`
  pub fn check(&self, index: usize, timing: Option<Timing>) -> Result<(), SceneError> {
    if timing.is_none() && self.is_animated() {
      return Err(SceneError::NoTiming { layer: self.label(index) });
    }
    let traversals = self.graph.as_ref().map_or(&[][..], |mask| &mask.traversals);
    for (i, traversal) in traversals.iter().enumerate() {
      if traversals[..i].iter().any(|other| other.name == traversal.name) {
        return Err(SceneError::SameTraversalName { layer: self.label(index), traversal: traversal.name.clone() });
      }
      if let Speed::Rate(Prop::Fixed(amount)) | Speed::Duration(Some(amount)) = traversal.speed
        && !(amount.is_finite() && amount > 0.0)
      {
        return Err(SceneError::NoSpeed { layer: self.label(index), traversal: traversal.name.clone() });
      }
    }
    Ok(())
  }

  /// The color at a sample. A hidden layer draws nothing. Without a shape it's clipped to the viewport; a shape can
  /// extend past it.
  pub fn sample(&self, at: &Sample, frame: &LayerFrame) -> Rgba<u8> {
    if self.hidden || (self.graph.is_some() && at.node.is_none()) {
      return Rgba([0, 0, 0, 0]);
    }
    let visits = frame.visits.zip(at.node).map(|(visits, node)| visits.at(node));
    let env = Env { lap: lap(frame.surface, at.radius), visits, ..env(frame) };
    let mut color = sample_fill(&self.fill, self.region.as_ref(), at, env.viewport, &env);
    if let Some(mask) = self.mask.as_ref().filter(|_| color[3] > 0) {
      color[3] = (color[3] as f32 * mask.amount(at, env.viewport, &env)).round() as u8;
    }
    color
  }

  /// The traversals of the layer's graph for the current cycle, to pass in [`LayerFrame::visits`]. `graph` is built
  /// from the points the layer is sampled at.
  ///
  /// Traversals are planned a cycle at a time, so with fixed starts and rates this only changes from one cycle (or
  /// playback seed) to the next, and can be kept for the rest of the cycle.
  pub fn visits(&self, graph: &Graph, frame: &LayerFrame) -> Visits {
    let env = env(frame);
    let traversals = self.graph.as_ref().map_or(&[][..], |mask| &mask.traversals);
    Visits { traversals: traversals.iter().map(|traversal| traversal.visits(graph, &env)).collect() }
  }

  /// The area the layer draws in, as a closed polygon in scene units, for editor overlays. With no shape, that's the
  /// viewport. A collapsed shape has no points.
  pub fn outline(&self, frame: &LayerFrame) -> Vec<Vec2> {
    let env = env(frame);
    fill_outline(self.region.as_ref(), env.viewport, &env)
  }

  /// The [`ShapePose`] of the layer's shape, for editor handles. `None` with no shape, or a collapsed one.
  pub fn shape_pose(&self, frame: &LayerFrame) -> Option<ShapePose> {
    self.region.as_ref()?.pose(&env(frame))
  }

  /// Like [`Self::outline`], for the layer's mask. No points with no mask.
  pub fn mask_outline(&self, frame: &LayerFrame) -> Vec<Vec2> {
    let env = env(frame);
    self.mask.as_ref().map_or_else(Vec::new, |mask| mask.outline(env.viewport, &env))
  }

  /// Like [`Self::shape_pose`], for the mask's shape
  pub fn mask_pose(&self, frame: &LayerFrame) -> Option<ShapePose> {
    self.mask.as_ref()?.region.as_ref()?.pose(&env(frame))
  }
}

/// The resolve environment for a frame. With no sample radius to go on, `lap` assumes points dense enough that their
/// spacing doesn't matter.
pub(crate) fn env<'a>(frame: &LayerFrame<'a>) -> Env<'a> {
  Env {
    time: frame.timing.map_or(0.0, |timing| timing.local_time(frame.time)),
    cycle: frame.timing.map_or(0, |timing| timing.cycle_index(frame.time)),
    timing: frame.timing,
    playback_seed: frame.playback_seed,
    params: frame.params,
    defaults: frame.defaults,
    images: frame.images,
    viewport: frame.surface.viewport(frame.size),
    lap: lap(frame.surface, 0.0),
    visits: None,
    fill_frame: Affine2::IDENTITY,
  }
}

/// Lap length of a `Sequence` for `Rotate`: the line plus one point's width to close the loop, so the last point doesn't
/// land on the first when the ends wrap.
fn lap(surface: Surface, radius: f32) -> Option<f32> {
  (surface == Surface::Sequence).then_some(1.0 + 2.0 * radius)
}

/// The fill's color at a sample, clipped to the region (or the viewport, with none). Edge coverage goes into alpha.
pub(crate) fn sample_fill(fill: &Fill, region: Option<&Shape>, at: &Sample, viewport: Bounds, env: &Env) -> Rgba<u8> {
  let (coverage, fill_pos, footprint) = match region {
    None if !viewport.contains(at.pos) => return Rgba([0, 0, 0, 0]),
    None => (1.0, viewport.to_unit(at.pos), viewport.to_unit_length(at.radius)),
    Some(shape) => match shape.cover(at, env) {
      Some((coverage, local, reach)) => {
        let bounds = shape.base.bounds(env);
        (coverage, bounds.to_unit(local), reach * bounds.to_unit_length(1.0))
      }
      None => return Rgba([0, 0, 0, 0]),
    },
  };

  let env = &Env { fill_frame: fill_frame(fill, region, viewport, env), ..*env };
  let mut color = fill.color(fill_pos, footprint, env);
  color[3] = (color[3] as f32 * coverage).round() as u8;
  color
}

/// Scene units to the unit box the fill is stretched over, for `Transform::Fit`. Identity for a fill that doesn't use it.
fn fill_frame(fill: &Fill, shape: Option<&Shape>, viewport: Bounds, env: &Env) -> Affine2 {
  if !fill.transform.iter().any(|transform| matches!(transform, Transform::Fit(_))) {
    return Affine2::IDENTITY;
  }
  match shape {
    None => viewport.to_unit_affine(),
    Some(shape) => {
      let bounds = shape.base.bounds(env);
      shape.to_local(bounds.center(), env).map_or(Affine2::IDENTITY, |to_local| bounds.to_unit_affine() * to_local)
    }
  }
}

/// The area a fill is drawn in, as a closed polygon in scene units
pub(crate) fn fill_outline(shape: Option<&Shape>, viewport: Bounds, env: &Env) -> Vec<Vec2> {
  match shape {
    None => {
      let Bounds { min, max } = viewport;
      vec![min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)]
    }
    Some(shape) => shape.outline(env),
  }
}
