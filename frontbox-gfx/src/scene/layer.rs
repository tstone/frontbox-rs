use glam::{Affine2, Vec2};
use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::color::RgbaColor;
use crate::layout::Grid;
use crate::scene::{Bounds, Env, Fill, LayerFrame, Graph, GraphMask, Mask, PreparedFill, PreparedMask, PreparedShape, Sample, SceneError, Shape, ShapePose, Speed, Surface, Timing, Transform, Visits, Prop};

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
  ///
  /// To sample many points in one frame, [`Self::prepare`] once and sample that.
  pub fn sample(&self, at: &Sample, frame: &LayerFrame) -> Rgba<u8> {
    self.prepare(frame, at.radius).sample(at)
  }

  /// The layer resolved for `frame`, for samples of `radius`. Its props are resolved here rather than at each sample,
  /// except for a layer with a graph, whose visits differ from node to node.
  pub fn prepare<'a>(&'a self, frame: &LayerFrame<'a>, radius: f32) -> PreparedLayer<'a> {
    let by_node = self.graph.is_some() && frame.visits.is_some();
    let env = Env { lap: lap(frame.surface, radius), ..env(frame) };
    let drawn = (!self.hidden && !by_node).then(|| Drawn::new(self, radius, &env));
    PreparedLayer { layer: self, frame: *frame, radius, drawn }
  }

  /// Draws the layer over `pixels`, a row at a time from the top left of `grid`
  pub fn sample_grid(&self, grid: &Grid, frame: &LayerFrame, pixels: &mut [Rgba<u8>]) {
    debug_assert_eq!(pixels.len(), grid.width * grid.height);
    let mut samples = grid.samples().peekable();
    let Some(first) = samples.peek() else {
      return;
    };
    let layer = self.prepare(frame, first.radius);
    for (pixel, at) in pixels.iter_mut().zip(samples) {
      *pixel = layer.sample(&at).composite_over(*pixel);
    }
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

  /// The area the layer draws in, as closed polygons in scene units for editor overlays: one for each copy of its shape.
  /// With no shape, that's the viewport. A collapsed shape has none.
  pub fn outline(&self, frame: &LayerFrame) -> Vec<Vec<Vec2>> {
    let env = env(frame);
    fill_outline(self.region.as_ref(), env.viewport, &env)
  }

  /// The [`ShapePose`] of the layer's shape, for editor handles. `None` with no shape, or a collapsed one.
  pub fn shape_pose(&self, frame: &LayerFrame) -> Option<ShapePose> {
    self.region.as_ref()?.pose(&env(frame))
  }

  /// Like [`Self::outline`], for the layer's mask. None with no mask.
  pub fn mask_outline(&self, frame: &LayerFrame) -> Vec<Vec<Vec2>> {
    let env = env(frame);
    self.mask.as_ref().map_or_else(Vec::new, |mask| mask.outline(env.viewport, &env))
  }

  /// Like [`Self::shape_pose`], for the mask's shape
  pub fn mask_pose(&self, frame: &LayerFrame) -> Option<ShapePose> {
    self.mask.as_ref()?.region.as_ref()?.pose(&env(frame))
  }
}

/// A [`Layer`] resolved for one frame, from [`Layer::prepare`]
pub struct PreparedLayer<'a> {
  layer: &'a Layer,
  frame: LayerFrame<'a>,
  /// The sample radius it was prepared for
  radius: f32,
  /// `None` for a hidden layer, or one prepared again at each node
  drawn: Option<Drawn<'a>>,
}

impl PreparedLayer<'_> {
  /// The color at a sample, as [`Layer::sample`] gives it. A sample of a radius other than the one prepared for is
  /// drawn too, but slower.
  pub fn sample(&self, at: &Sample) -> Rgba<u8> {
    if self.layer.hidden || (self.layer.graph.is_some() && at.node.is_none()) {
      return Rgba([0, 0, 0, 0]);
    }
    match &self.drawn {
      Some(drawn) if at.radius == self.radius => drawn.sample(at),
      _ => {
        let visits = self.frame.visits.zip(at.node).map(|(visits, node)| visits.at(node));
        let env = Env { lap: lap(self.frame.surface, at.radius), visits, ..env(&self.frame) };
        Drawn::new(self.layer, at.radius, &env).sample(at)
      }
    }
  }
}

/// What a layer draws, resolved for samples of one radius
struct Drawn<'a> {
  /// `None` if the region is collapsed
  fill: Option<PreparedArea<'a>>,
  mask: Option<PreparedMask<'a>>,
}

impl<'a> Drawn<'a> {
  fn new(layer: &'a Layer, radius: f32, env: &Env<'a>) -> Self {
    Self {
      fill: prepare_area(&layer.fill, layer.region.as_ref(), env.viewport, radius, env),
      mask: layer.mask.as_ref().map(|mask| mask.prepare(env.viewport, radius, env)),
    }
  }

  fn sample(&self, at: &Sample) -> Rgba<u8> {
    let Some(fill) = &self.fill else {
      return Rgba([0, 0, 0, 0]);
    };
    let mut color = fill.color(at);
    if let Some(mask) = self.mask.as_ref().filter(|_| color[3] > 0) {
      color[3] = (color[3] as f32 * mask.amount(at)).round() as u8;
    }
    color
  }
}

/// A fill clipped to its region, or to the viewport with none, resolved for samples of one radius
pub(crate) struct PreparedArea<'a> {
  clip: Clip,
  fill: PreparedFill<'a>,
}

impl PreparedArea<'_> {
  /// The fill's color at a sample, with edge coverage in alpha
  pub(crate) fn color(&self, at: &Sample) -> Rgba<u8> {
    let (coverage, fill_pos) = match &self.clip {
      Clip::Viewport(viewport) if !viewport.contains(at.pos) => return Rgba([0, 0, 0, 0]),
      Clip::Viewport(viewport) => (1.0, viewport.to_unit(at.pos)),
      Clip::Shape(shape) => match shape.cover(at) {
        Some((coverage, local)) => (coverage, shape.bounds.to_unit(local)),
        None => return Rgba([0, 0, 0, 0]),
      },
    };
    let mut color = self.fill.color(fill_pos);
    color[3] = (color[3] as f32 * coverage).round() as u8;
    color
  }
}

/// What a fill is clipped to
enum Clip {
  Viewport(Bounds),
  Shape(PreparedShape),
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

/// The fill clipped to the region (or the viewport, with none), for samples of `radius`. `None` if either is collapsed,
/// so it draws nothing.
pub(crate) fn prepare_area<'a>(fill: &'a Fill, region: Option<&Shape>, viewport: Bounds, radius: f32, env: &Env<'a>) -> Option<PreparedArea<'a>> {
  let (clip, footprint) = match region {
    None => (Clip::Viewport(viewport), viewport.to_unit_length(radius)),
    Some(shape) => {
      let shape = shape.prepare(env)?;
      let footprint = shape.to_local.matrix2.abs() * Vec2::splat(radius) * shape.bounds.to_unit_length(1.0);
      (Clip::Shape(shape), footprint)
    }
  };
  let env = &Env { fill_frame: fill_frame(fill, &clip), ..*env };
  Some(PreparedArea { fill: fill.prepare(footprint, env)?, clip })
}

/// Scene units to the unit box the fill is stretched over, for `Transform::Fit`. Identity for a fill that doesn't use it.
fn fill_frame(fill: &Fill, clip: &Clip) -> Affine2 {
  if !fill.transform.iter().any(|transform| matches!(transform, Transform::Fit(_))) {
    return Affine2::IDENTITY;
  }
  match clip {
    Clip::Viewport(viewport) => viewport.to_unit_affine(),
    Clip::Shape(shape) => shape.bounds.to_unit_affine() * shape.to_local,
  }
}

/// The area a fill is drawn in, as closed polygons in scene units
pub(crate) fn fill_outline(shape: Option<&Shape>, viewport: Bounds, env: &Env) -> Vec<Vec<Vec2>> {
  match shape {
    None => {
      let Bounds { min, max } = viewport;
      vec![vec![min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)]]
    }
    Some(shape) => shape.outline(env),
  }
}
