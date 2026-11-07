use std::fmt;

use glam::Vec2;
use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::animation::Cycle;
use crate::color::RgbaColor;
use crate::scene::{Env, Frame, Layer, Params, Sample, ShapePose, Timing, Viewport, file};

/// How far inside a cycle in a repeating scene its first and last moments are, as a fraction of the cycle. Any nearer and
/// the time rounds into the cycle next to it.
const CYCLE_MARGIN: f32 = 1e-4;

/// A graphic and how it moves over time: layers seen through a viewport, and the parameters the game can change. Ready to be
/// sampled by any device, and written to a file with [`file`].
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
  pub version: u32,
  /// Parameter names and their defaults
  #[serde(default, skip_serializing_if = "Params::is_empty")]
  #[cfg_attr(feature = "ts", ts(as = "Option<Params>", optional))]
  pub params: Params,
  pub viewport: Viewport,
  /// `None` is a static scene, whose tracks stay on their first moment
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub timing: Option<Timing>,
  /// Names for the separate surfaces it plays on at once, which layers choose between. What each one is on comes from
  /// the device playing the scene, and they all share its timing. With none, it plays on one surface.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
  pub targets: Vec<String>,
  /// Drawn bottom to top, alpha over
  #[serde(default)]
  pub layers: Vec<Layer>,
}

impl Scene {
  /// A scene of the current format version
  pub fn new(viewport: Viewport, layers: Vec<Layer>) -> Self {
    Self { version: file::VERSION, params: Params::new(), viewport, timing: None, targets: Vec::new(), layers }
  }

  pub fn timing(mut self, timing: Timing) -> Self {
    self.timing = Some(timing);
    self
  }

  pub fn targets(mut self, targets: impl IntoIterator<Item = impl Into<String>>) -> Self {
    self.targets = targets.into_iter().map(Into::into).collect();
    self
  }

  /// This scene with only the layers that play on `target`
  pub fn for_target(&self, target: &str) -> Self {
    let mut scene = self.clone();
    scene.layers.retain(|layer| layer.plays_on(target));
    scene
  }

  /// Whether every cycle of the scene has played by `time`. A scene that's static or repeats forever never completes.
  pub fn is_complete(&self, time: f32) -> bool {
    self.timing.is_some_and(|timing| timing.is_complete(time))
  }

  /// This scene for showing a cycle as it looks partway through play, with a cycle either side to reach into, such as for
  /// random picks to smooth across. A scene that plays one cycle has none either side, and stays as it is.
  pub fn middle_cycle(&self) -> Self {
    let mut scene = self.clone();
    if let Some(timing) = scene.timing.as_mut().filter(|timing| timing.repeats()) {
      timing.cycle = Cycle::Times(3);
    }
    scene
  }

  /// The time in a scene from [`Scene::middle_cycle`] of a time in the cycle it shows, up to and including the end
  pub fn middle_cycle_time(&self, time: f32) -> f32 {
    self.cycle_time(1, time)
  }

  /// The time in the scene of a time in one of its cycles, up to and including the end of it. A cycle past the scene's
  /// last is its last.
  pub fn cycle_time(&self, cycle: u32, time: f32) -> f32 {
    match self.timing {
      Some(timing) if timing.repeats() => {
        let cycle = match timing.cycle {
          Cycle::Times(times) => cycle.min(times - 1),
          _ => cycle,
        };
        timing.duration * (cycle as f32 + (time / timing.duration).clamp(CYCLE_MARGIN, 1.0 - CYCLE_MARGIN))
      }
      _ => time,
    }
  }

  /// Checks what a definition's types can't say for themselves. Scenes from outside the program need this; [`file::load`]
  /// already does it.
  pub fn check(&self) -> Result<(), SceneError> {
    match (self.timing, self.layers.iter().position(Layer::is_animated)) {
      (Some(timing), _) if !(timing.duration.is_finite() && timing.duration > 0.0) => return Err(SceneError::NoDuration),
      (None, Some(index)) => return Err(SceneError::NoTiming { layer: self.layers[index].label(index) }),
      _ => {}
    }
    if let Some((_, name)) = self.targets.iter().enumerate().find(|(index, name)| self.targets[..*index].contains(name)) {
      return Err(SceneError::RepeatedTarget { target: name.clone() });
    }
    for (index, layer) in self.layers.iter().enumerate() {
      if let Some(target) = layer.targets.iter().flatten().find(|target| !self.targets.contains(target)) {
        return Err(SceneError::UnknownTarget { layer: layer.label(index), target: target.clone() });
      }
    }
    Ok(())
  }

  /// The names of the images it draws, which can repeat
  pub fn image_names(&self) -> impl Iterator<Item = &str> {
    self.layers.iter().flat_map(Layer::image_names)
  }

  /// The color at a sample. Transparent outside the viewport.
  pub fn sample(&self, at: &Sample, frame: &Frame) -> Rgba<u8> {
    let env = Env { lap: self.lap(at.radius), ..self.env(frame) };
    let viewport = self.viewport.bounds(frame.size);
    if !viewport.contains(at.pos) {
      return Rgba([0, 0, 0, 0]);
    }
    self
      .layers
      .iter()
      .filter(|layer| !layer.hidden)
      .fold(Rgba([0, 0, 0, 0]), |below, layer| layer.sample(at, viewport, &env).composite_over(below))
  }

  /// Where each layer is drawn, as a closed loop of points in scene units. A layer with no shape outlines the viewport,
  /// and one that draws nothing has no points. A hidden layer has its outline all the same, for going back to edit it.
  pub fn outlines(&self, frame: &Frame) -> Vec<Vec<Vec2>> {
    let env = self.env(frame);
    let viewport = self.viewport.bounds(frame.size);
    self.layers.iter().map(|layer| layer.outline(viewport, &env)).collect()
  }

  /// Each layer's shape as it stands at a moment. `None` for a layer with no shape, or one that a transform collapses.
  pub fn shape_poses(&self, frame: &Frame) -> Vec<Option<ShapePose>> {
    let env = self.env(frame);
    self.layers.iter().map(|layer| layer.shape.as_ref().and_then(|shape| shape.pose(&env))).collect()
  }

  /// Where each layer's mask is drawn, in the same order and terms as [`Scene::outlines`]. A layer with no mask has no
  /// points.
  pub fn mask_outlines(&self, frame: &Frame) -> Vec<Vec<Vec2>> {
    let env = self.env(frame);
    let viewport = self.viewport.bounds(frame.size);
    self.layers.iter().map(|layer| layer.mask.as_ref().map_or_else(Vec::new, |mask| mask.outline(viewport, &env))).collect()
  }

  /// Each layer's mask's shape as it stands at a moment. `None` for a layer with no such shape, or one that a transform
  /// collapses.
  pub fn mask_poses(&self, frame: &Frame) -> Vec<Option<ShapePose>> {
    let env = self.env(frame);
    self.layers.iter().map(|layer| layer.mask.as_ref()?.shape.as_ref()?.pose(&env)).collect()
  }

  /// How far it is around a `Sequence` of points this far across: the line and the gap that closes it, which spaces the
  /// points evenly around a ring, so that the last never lands on the first.
  fn lap(&self, radius: f32) -> Option<f32> {
    (self.viewport == Viewport::Sequence).then(|| 1.0 + 2.0 * radius)
  }

  /// Without a sample to go by, a `Sequence` is taken to be too many points for their spacing to matter
  fn env<'a>(&'a self, frame: &Frame<'a>) -> Env<'a> {
    let time = self.timing.map_or(0.0, |timing| timing.local_time(frame.time));
    let cycle = self.timing.map_or(0, |timing| timing.cycle_index(frame.time));
    Env {
      time,
      cycle,
      timing: self.timing,
      take: frame.take,
      params: frame.params,
      defaults: &self.params,
      images: frame.images,
      viewport: self.viewport.bounds(frame.size),
      lap: self.lap(0.0),
    }
  }
}

/// Why a scene can't be played
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneError {
  /// A layer has props that change over time, but there's no timing to play them in
  NoTiming { layer: String },
  /// Its timing has no length
  NoDuration,
  /// Two of its targets have the same name
  RepeatedTarget { target: String },
  /// A layer plays on a target the scene doesn't have
  UnknownTarget { layer: String, target: String },
}

impl fmt::Display for SceneError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NoTiming { layer } => write!(f, "{layer} changes over time, so the scene needs a timing"),
      Self::NoDuration => write!(f, "a scene's duration must be more than zero"),
      Self::RepeatedTarget { target } => write!(f, "there are two targets named \"{target}\""),
      Self::UnknownTarget { layer, target } => write!(f, "{layer} plays on \"{target}\", which isn't one of the scene's targets"),
    }
  }
}

impl std::error::Error for SceneError {}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::animation::{Curve, Cycle};
  use crate::layout::{Grid, Line};
  use crate::scene::{
    Color, ColorAdjust, ColorSpace, Fill, FillNode, GradientStyle, Images, Key, Mask, Pick, Picture, Prop, Random, Shape, ShapeNode, Spread,
    Stop, Track, Transform,
  };

  const SIZE: Vec2 = Vec2::new(100.0, 50.0);
  const RED: Rgba<u8> = Rgba([255, 0, 0, 255]);
  const BLUE: Rgba<u8> = Rgba([0, 0, 255, 255]);
  const DARK: Rgba<u8> = Rgba([16, 16, 16, 255]);

  fn solid(color: Rgba<u8>) -> Fill {
    Fill::new(FillNode::Solid(Color(color).into()))
  }

  fn red_to_blue() -> Fill {
    Fill::new(FillNode::Gradient {
      stops: vec![Stop::new(0.0, Color(RED)), Stop::new(1.0, Color(BLUE))],
      space: ColorSpace::Rgb,
      style: GradientStyle::Linear,
    })
  }

  fn sample(scene: &Scene, x: f32, y: f32, radius: f32) -> Rgba<u8> {
    let params = Params::new();
    let frame = Frame { time: 0.0, take: 0, params: &params, images: Images::none(), size: SIZE };
    scene.sample(&Sample::new(Vec2::new(x, y), radius), &frame)
  }

  fn no_timing(layer: &str) -> SceneError {
    SceneError::NoTiming { layer: layer.into() }
  }

  #[test]
  fn a_scene_that_cant_play_says_which_layer_is_why() {
    let changing = || solid(RED).adjust(ColorAdjust::HueShift(Prop::Track(Track { keys: vec![], cycle: Cycle::Once })));
    let scene = |top: Layer| Scene::new(Viewport::Area, vec![Layer::new(solid(RED)), top]);

    let error = scene(Layer::new(changing()).name("sparkle")).check().unwrap_err();
    assert_eq!(error, no_timing("layer \"sparkle\""));
    assert_eq!(error.to_string(), "layer \"sparkle\" changes over time, so the scene needs a timing");
    assert_eq!(scene(Layer::new(changing())).check(), Err(no_timing("layer 2")));
  }

  #[test]
  fn a_hidden_layer_isnt_drawn_and_keeps_its_place() {
    let scene = Scene::new(Viewport::Area, vec![Layer::new(solid(RED)), Layer::new(solid(BLUE)).hidden()]);
    assert_eq!(sample(&scene, 1.0, 1.0, 0.5), RED);

    let params = Params::new();
    let frame = Frame { time: 0.0, take: 0, params: &params, images: Images::none(), size: SIZE };
    assert_eq!(scene.outlines(&frame).len(), 2);
  }

  fn turned(degrees: f32) -> Scene {
    Scene::new(Viewport::Sequence, vec![Layer::new(red_to_blue().transform(Transform::Rotate(degrees.into())))])
  }

  #[test]
  fn rotate_runs_a_sequence_around_a_point_at_a_time() {
    let still = line(&turned(0.0), 8);
    let mut around = still.clone();
    around.rotate_right(1);
    assert_eq!(line(&turned(45.0), 8), around);
    around.rotate_left(2);
    assert_eq!(line(&turned(-45.0), 8), around);
  }

  #[test]
  fn a_lap_of_a_sequence_comes_back_to_the_start() {
    assert_eq!(line(&turned(360.0), 8), line(&turned(0.0), 8));
    assert_eq!(line(&turned(-720.0), 8), line(&turned(0.0), 8));
  }

  #[test]
  fn rotate_runs_a_shape_past_the_end_of_a_sequence_to_its_start() {
    let lit = |transform: Vec<Transform>| -> Vec<bool> {
      let shape = transform.into_iter().fold(Shape::new(ShapeNode::Rect { width: 0.1.into(), height: 1.0.into() }), Shape::transform);
      line(&Scene::new(Viewport::Sequence, vec![Layer::new(solid(RED)).shape(shape)]), 5).iter().map(|color| color[3] > 0).collect()
    };
    let place = || Transform::Translate(Vec2::new(0.75, 0.0).into());
    let turn = |degrees: f32| Transform::Rotate(degrees.into());

    assert_eq!(lit(vec![place(), turn(72.0)]), [false, false, false, false, true]);
    assert_eq!(lit(vec![place(), turn(144.0)]), [true, false, false, false, false]);
    assert_eq!(lit(vec![turn(144.0), place()]), [true, false, false, false, false]);
  }

  #[test]
  fn a_shape_rotating_on_an_area_turns_rather_than_runs_around() {
    let bar = Shape::new(ShapeNode::Rect { width: 40.0.into(), height: 2.0.into() })
      .transform(Transform::Rotate(90.0.into()))
      .transform(Transform::Translate(Vec2::new(50.0, 25.0).into()));
    let scene = Scene::new(Viewport::Area, vec![Layer::new(solid(RED)).shape(bar)]);
    assert_eq!(sample(&scene, 50.0, 10.0, 0.1), RED);
    assert_eq!(sample(&scene, 65.0, 25.0, 0.1)[3], 0);
  }

  #[test]
  fn a_shape_moved_then_rotated_turns_where_it_is() {
    let bar = Shape::new(ShapeNode::Rect { width: 40.0.into(), height: 2.0.into() })
      .transform(Transform::Translate(Vec2::new(50.0, 25.0).into()))
      .transform(Transform::Rotate(90.0.into()));
    let scene = Scene::new(Viewport::Area, vec![Layer::new(solid(RED)).shape(bar)]);
    assert_eq!(sample(&scene, 50.0, 10.0, 0.1), RED);
    assert_eq!(sample(&scene, 65.0, 25.0, 0.1)[3], 0);
  }

  #[test]
  fn a_polygon_away_from_the_origin_turns_around_its_own_middle() {
    let bar = ShapeNode::Polygon {
      points: [(10.0, 4.0), (30.0, 4.0), (30.0, 6.0), (10.0, 6.0)].map(|(x, y)| Vec2::new(x, y).into()).to_vec(),
    };
    let shape = Shape::new(bar).transform(Transform::Rotate(90.0.into()));
    let scene = Scene::new(Viewport::Area, vec![Layer::new(solid(RED)).shape(shape)]);
    assert_eq!(sample(&scene, 20.0, 12.0, 0.1), RED);
    assert_eq!(sample(&scene, 12.0, 5.0, 0.1)[3], 0);
  }

  fn line(scene: &Scene, count: usize) -> Vec<Rgba<u8>> {
    let params = Params::new();
    let frame = Frame { time: 0.0, take: 0, params: &params, images: Images::none(), size: SIZE };
    Line::new(count).samples().map(|s| scene.sample(&s, &frame)).collect()
  }

  /// A scene a second long whose color is picked at the start of each cycle
  fn random_color(cycle: Cycle) -> Scene {
    let spread = Spread::Color { hue: (0.0, 360.0), brightness: (1.0, 1.0), alpha: (1.0, 1.0) };
    let color = Prop::Random(Random { picks: vec![Pick::new(0.0)], spread, seed: 1, force_unique: true });
    Scene::new(Viewport::Sequence, vec![Layer::new(Fill::new(FillNode::Solid(color)))]).timing(Timing::new(1.0, cycle))
  }

  fn random_color_at(scene: &Scene, time: f32, take: u32) -> Rgba<u8> {
    let params = Params::new();
    let frame = Frame { time, take, params: &params, images: Images::none(), size: SIZE };
    scene.sample(&Sample::new(Vec2::new(0.5, 0.0), 0.5), &frame)
  }

  #[test]
  fn a_scene_that_repeats_has_a_cycle_either_side_of_its_middle() {
    let middle = |cycle| Scene::new(Viewport::Sequence, vec![]).timing(Timing::new(1.0, cycle)).middle_cycle();
    assert_eq!(middle(Cycle::Forever).timing.unwrap().cycle, Cycle::Times(3));
    assert_eq!(middle(Cycle::Times(2)).timing.unwrap().cycle, Cycle::Times(3));
    assert_eq!(middle(Cycle::Forever).middle_cycle_time(0.5), 1.5);
  }

  #[test]
  fn a_scene_of_one_cycle_stays_as_it_is_in_its_middle() {
    for cycle in [Cycle::Once, Cycle::Times(1)] {
      let scene = Scene::new(Viewport::Sequence, vec![]).timing(Timing::new(1.0, cycle)).middle_cycle();
      assert_eq!(scene.timing.unwrap().cycle, cycle);
      assert_eq!(scene.middle_cycle_time(0.5), 0.5);
    }
  }

  #[test]
  fn the_end_of_a_middle_cycle_is_still_that_cycle() {
    let scene = Scene::new(Viewport::Sequence, vec![]).timing(Timing::new(0.1, Cycle::Forever)).middle_cycle();
    assert_eq!(scene.timing.unwrap().cycle_index(scene.middle_cycle_time(0.1)), 1);
  }

  #[test]
  fn a_time_in_a_cycle_stays_in_that_cycle() {
    for duration in [0.1, 0.7, 2.0] {
      let timing = Timing::new(duration, Cycle::Forever);
      let scene = Scene::new(Viewport::Sequence, vec![]).timing(timing);
      for cycle in 0..1000 {
        assert_eq!(timing.cycle_index(scene.cycle_time(cycle, 0.0)), cycle, "start of {cycle} of {duration}");
        assert_eq!(timing.cycle_index(scene.cycle_time(cycle, duration)), cycle, "end of {cycle} of {duration}");
      }
    }
  }

  #[test]
  fn a_cycle_past_the_last_is_the_last() {
    let scene = Scene::new(Viewport::Sequence, vec![]).timing(Timing::new(1.0, Cycle::Times(3)));
    assert_eq!(scene.cycle_time(7, 0.5), 2.5);
  }

  #[test]
  fn a_scene_of_one_cycle_has_only_that_one() {
    let scene = Scene::new(Viewport::Sequence, vec![]).timing(Timing::new(1.0, Cycle::Once));
    assert_eq!(scene.cycle_time(3, 1.0), 1.0);
  }

  #[test]
  fn random_picks_smooth_into_the_next_cycle() {
    let smoothed = |cycle| {
      let spread = Spread::Float(0.0, 360.0);
      let pick = Pick { at: 0.0, smooth_in: 1.0, smooth_out: 0.0 };
      let shift = Prop::Random(Random { picks: vec![pick], spread, seed: 1, force_unique: true });
      Scene::new(Viewport::Sequence, vec![Layer::new(solid(RED).adjust(ColorAdjust::HueShift(shift)))])
        .timing(Timing::new(1.0, cycle))
    };
    let looping = smoothed(Cycle::Forever);
    assert_ne!(random_color_at(&looping, 1.25, 0), random_color_at(&looping, 1.75, 0));
    let once = smoothed(Cycle::Once);
    assert_eq!(random_color_at(&once, 0.25, 0), random_color_at(&once, 0.75, 0));
  }

  #[test]
  fn random_picks_change_each_cycle_and_hold_within_one() {
    let scene = random_color(Cycle::Forever);
    assert_eq!(random_color_at(&scene, 0.1, 0), random_color_at(&scene, 0.9, 0));
    assert_ne!(random_color_at(&scene, 0.9, 0), random_color_at(&scene, 1.1, 0));
    assert_eq!(random_color_at(&scene, 1.1, 0), random_color_at(&scene, 1.9, 0));
  }

  #[test]
  fn a_take_is_the_same_each_time_and_differs_from_another() {
    let scene = random_color(Cycle::Forever);
    assert_eq!(random_color_at(&scene, 0.5, 3), random_color_at(&scene, 0.5, 3));
    assert_ne!(random_color_at(&scene, 0.5, 3), random_color_at(&scene, 0.5, 4));
  }

  #[test]
  fn a_completed_scene_holds_its_last_pick() {
    let scene = random_color(Cycle::Times(2));
    assert_eq!(random_color_at(&scene, 1.9, 0), random_color_at(&scene, 2.0, 0));
    assert_eq!(random_color_at(&scene, 1.9, 0), random_color_at(&scene, 30.0, 0));
  }

  #[test]
  fn random_props_need_a_timing() {
    let mut scene = random_color(Cycle::Once);
    scene.timing = None;
    assert_eq!(scene.check().unwrap_err(), no_timing("layer 1"));
  }

  /// A circle of diameter 40 at (50, 25) in a 100 x 50 viewport, over a dark background
  fn circle(fill: Fill) -> Scene {
    Scene::new(Viewport::Area, vec![
      Layer::new(solid(DARK)),
      Layer::new(fill).shape(
        Shape::new(ShapeNode::circle(0.5))
          .transform(Transform::Scale(Vec2::splat(40.0).into()))
          .transform(Transform::Translate(Vec2::new(50.0, 25.0).into())),
      ),
    ])
  }

  fn outlines(layer: Layer) -> Vec<Vec2> {
    let params = Params::new();
    let frame = Frame { time: 0.0, take: 0, params: &params, images: Images::none(), size: SIZE };
    Scene::new(Viewport::Area, vec![layer]).outlines(&frame).remove(0)
  }

  #[test]
  fn a_device_decides_the_size_of_an_area() {
    let scene = Scene::new(Viewport::Area, vec![Layer::new(solid(RED))]);
    let params = Params::new();
    let at = Sample::new(Vec2::new(150.0, 25.0), 0.0);
    let color = |size: Vec2| scene.sample(&at, &Frame { time: 0.0, take: 0, params: &params, images: Images::none(), size });
    assert_eq!(color(Vec2::new(200.0, 50.0)), RED);
    assert_eq!(color(SIZE), Rgba([0, 0, 0, 0]));
  }

  #[test]
  fn a_gradient_with_no_shape_stretches_to_the_device() {
    let scene = Scene::new(Viewport::Area, vec![Layer::new(red_to_blue())]);
    let params = Params::new();
    let far_edge = |width: f32| {
      let frame = Frame { time: 0.0, take: 0, params: &params, images: Images::none(), size: Vec2::new(width, 50.0) };
      scene.sample(&Sample::new(Vec2::new(width, 25.0), 0.0), &frame)
    };
    assert_eq!(far_edge(100.0), BLUE);
    assert_eq!(far_edge(300.0), BLUE);
  }

  #[test]
  fn a_layer_with_no_shape_outlines_the_viewport() {
    let corners = [(0.0, 0.0), (100.0, 0.0), (100.0, 50.0), (0.0, 50.0)].map(|(x, y)| Vec2::new(x, y));
    assert_eq!(outlines(Layer::new(solid(RED))), corners);
  }

  #[test]
  fn an_outline_follows_a_shapes_transforms_in_order() {
    let square = ShapeNode::Polygon {
      points: [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| Vec2::new(x, y).into()).to_vec(),
    };
    let shape = Shape::new(square)
      .transform(Transform::Scale(Vec2::new(2.0, 4.0).into()))
      .transform(Transform::Translate(Vec2::new(10.0, 20.0).into()));
    let corners = [(10.0, 20.0), (12.0, 20.0), (12.0, 24.0), (10.0, 24.0)].map(|(x, y)| Vec2::new(x, y));
    assert_eq!(outlines(Layer::new(solid(RED)).shape(shape)), corners);
  }

  #[test]
  fn an_ellipse_outline_runs_around_its_edge() {
    let shape = Shape::new(ShapeNode::circle(0.5))
      .transform(Transform::Scale(Vec2::new(40.0, 20.0).into()))
      .transform(Transform::Translate(Vec2::new(50.0, 25.0).into()));
    let outline = outlines(Layer::new(solid(RED)).shape(shape));
    assert!(outline.len() > 16);
    for point in outline {
      let offset = (point - Vec2::new(50.0, 25.0)) / Vec2::new(20.0, 10.0);
      assert!((offset.length() - 1.0).abs() < 1e-4, "{point}");
    }
  }

  #[test]
  fn a_shape_that_draws_nothing_has_no_outline() {
    let collapsed = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(Vec2::ZERO.into()));
    assert!(outlines(Layer::new(solid(RED)).shape(collapsed)).is_empty());
    let line = Shape::new(ShapeNode::Polygon { points: vec![Vec2::ZERO.into(), Vec2::ONE.into()] });
    assert!(outlines(Layer::new(solid(RED)).shape(line)).is_empty());
  }

  #[test]
  fn solid_on_a_line() {
    let scene = Scene::new(Viewport::Sequence, vec![Layer::new(solid(RED))]);
    assert_eq!(line(&scene, 4), vec![RED; 4]);
  }

  #[test]
  fn gradient_on_a_line_matches_color_sequence() {
    use crate::prelude::*;

    for count in [1, 2, 5, 16, 33] {
      let scene = Scene::new(Viewport::Sequence, vec![Layer::new(red_to_blue())]);
      let expected = ColorSequence::fade(RED, BLUE).generate(count);
      let actual = line(&scene, count);
      for (i, (a, e)) in actual.iter().zip(&expected).enumerate() {
        let close = (0..4).all(|c| (a[c] as i16 - e[c] as i16).abs() <= 1);
        assert!(close, "{count} LEDs, LED {i}: {a:?} vs ColorSequence {e:?}");
      }
    }
  }

  #[test]
  fn shapes_on_a_grid() {
    let scene = circle(solid(RED));
    // a 100 x 50 DMD, one sample per pixel
    let lit = (0..50)
      .flat_map(|y| (0..100).map(move |x| (x, y)))
      .map(|(x, y)| sample(&scene, x as f32 + 0.5, y as f32 + 0.5, 0.5))
      .filter(|c| *c == RED)
      .count();
    // close to the area of the circle, less the partly covered edge
    let area = std::f32::consts::PI * 20.0 * 20.0;
    assert!((lit as f32 - area).abs() < 2.0 * std::f32::consts::PI * 20.0, "{lit} vs {area}");
    assert_eq!(sample(&scene, 50.0, 25.0, 0.5), RED);
    assert_eq!(sample(&scene, 5.0, 5.0, 0.5), DARK);
  }

  /// A circle of diameter 40 at (50, 25)
  fn middle_circle() -> Shape {
    Shape::new(ShapeNode::circle(0.5))
      .transform(Transform::Scale(Vec2::splat(40.0).into()))
      .transform(Transform::Translate(Vec2::new(50.0, 25.0).into()))
  }

  /// A red layer over a dark one, with a mask on the red
  fn masked(mask: Mask) -> Scene {
    Scene::new(Viewport::Area, vec![Layer::new(solid(DARK)), Layer::new(solid(RED)).mask(mask)])
  }

  #[test]
  fn a_mask_shows_its_layer_only_inside_it_and_leaves_the_layer_below() {
    let scene = masked(Mask::new(solid(BLUE)).shape(middle_circle()));
    assert_eq!(sample(&scene, 50.0, 25.0, 0.5), RED);
    assert_eq!(sample(&scene, 5.0, 5.0, 0.5), DARK);
  }

  #[test]
  fn an_inverted_mask_shows_its_layer_only_outside_it() {
    let scene = masked(Mask::new(solid(BLUE)).shape(middle_circle()).inverted());
    assert_eq!(sample(&scene, 50.0, 25.0, 0.5), DARK);
    assert_eq!(sample(&scene, 5.0, 5.0, 0.5), RED);
  }

  #[test]
  fn a_mask_with_no_shape_covers_the_viewport() {
    assert_eq!(sample(&masked(Mask::new(solid(BLUE))), 5.0, 5.0, 0.5), RED);
    assert_eq!(sample(&masked(Mask::new(solid(BLUE)).inverted()), 5.0, 5.0, 0.5), DARK);
  }

  #[test]
  fn a_mask_fades_its_layer_by_its_own_opacity() {
    let half = solid(BLUE).adjust(ColorAdjust::Opacity(0.5.into()));
    let scene = Scene::new(Viewport::Area, vec![Layer::new(solid(RED)).mask(Mask::new(half))]);
    let alpha = sample(&scene, 50.0, 25.0, 0.5)[3];
    assert!((alpha as i16 - 128).abs() <= 1, "{alpha}");
  }

  #[test]
  fn a_mask_stays_put_as_its_layer_moves() {
    let moved = middle_circle().transform(Transform::Translate(Vec2::new(20.0, 0.0).into()));
    let mask = Mask::new(solid(BLUE)).shape(middle_circle());
    let scene = Scene::new(Viewport::Area, vec![Layer::new(solid(RED)).shape(moved).mask(mask)]);
    // in both, in the layer alone, and in the mask alone
    assert_eq!(sample(&scene, 60.0, 25.0, 0.5), RED);
    assert_eq!(sample(&scene, 85.0, 25.0, 0.5)[3], 0);
    assert_eq!(sample(&scene, 35.0, 25.0, 0.5)[3], 0);
  }

  #[test]
  fn an_animated_mask_needs_a_timing() {
    let changing = solid(RED).adjust(ColorAdjust::HueShift(Prop::Track(Track { keys: vec![], cycle: Cycle::Once })));
    assert_eq!(masked(Mask::new(changing)).check(), Err(no_timing("layer 2")));
  }

  #[test]
  fn a_masks_image_is_one_the_scene_draws() {
    let image = Fill::new(FillNode::Image { name: "stencil.png".into(), frame: None });
    let scene = masked(Mask::new(image));
    assert_eq!(scene.image_names().collect::<Vec<_>>(), ["stencil.png"]);
  }

  #[test]
  fn a_masks_outline_and_pose_line_up_with_the_layers() {
    let scene = masked(Mask::new(solid(BLUE)).shape(middle_circle()));
    let params = Params::new();
    let frame = Frame { time: 0.0, take: 0, params: &params, images: Images::none(), size: SIZE };
    assert!(scene.mask_outlines(&frame)[0].is_empty());
    assert!(!scene.mask_outlines(&frame)[1].is_empty());
    assert!(scene.mask_poses(&frame)[0].is_none());
    assert!(scene.mask_poses(&frame)[1].is_some());
  }

  #[test]
  fn shapes_at_sparse_points() {
    let scene = circle(solid(RED));
    // inserts with a large footprint, one well inside, one well outside, one straddling the edge
    assert_eq!(sample(&scene, 50.0, 25.0, 3.0), RED);
    assert_eq!(sample(&scene, 90.0, 25.0, 3.0), DARK);
    let edge = sample(&scene, 70.0, 25.0, 3.0);
    assert!(edge[0] > DARK[0] && edge[0] < RED[0], "{edge:?}");
  }

  #[test]
  fn gradient_fills_its_shape() {
    let scene = circle(red_to_blue());
    assert_eq!(sample(&scene, 30.5, 25.0, 0.0), Rgba([252, 0, 3, 255]));
    assert_eq!(sample(&scene, 69.5, 25.0, 0.0), Rgba([3, 0, 252, 255]));
  }

  #[test]
  fn gradient_turns_with_its_shape() {
    let mut scene = circle(red_to_blue());
    let Some(shape) = &mut scene.layers[1].shape else { unreachable!() };
    shape.transform.insert(1, Transform::Rotate(90.0.into()));
    // the red end is now at the top, and the blue end at the bottom
    assert_eq!(sample(&scene, 50.0, 5.5, 0.0), Rgba([252, 0, 3, 255]));
    assert_eq!(sample(&scene, 50.0, 44.5, 0.0), Rgba([3, 0, 252, 255]));
  }

  #[test]
  fn shape_on_a_line_is_measured_along_it() {
    // on 5 LEDs, a circle most of the way across centered on the middle lights all but the end LEDs
    let scene = Scene::new(Viewport::Sequence, vec![Layer::new(solid(RED)).shape(
      Shape::new(ShapeNode::circle(0.5))
        .transform(Transform::Scale(Vec2::splat(0.75).into()))
        .transform(Transform::Translate(Vec2::new(0.5, 0.0).into())),
    )]);
    let transparent = Rgba([0, 0, 0, 0]);
    assert_eq!(line(&scene, 5), vec![transparent, RED, RED, RED, transparent]);
  }

  #[test]
  fn a_sequence_plays_on_any_number_of_leds() {
    let scene = Scene::new(Viewport::Sequence, vec![Layer::new(red_to_blue())]);
    for count in [2, 6, 8, 60] {
      let colors = line(&scene, count);
      assert_eq!(colors[0], RED, "{count} LEDs");
      assert_eq!(colors[count - 1], BLUE, "{count} LEDs");
    }
  }

  #[test]
  fn a_shape_covers_the_same_part_of_any_strip() {
    let scene = Scene::new(Viewport::Sequence, vec![Layer::new(solid(RED)).shape(
      Shape::new(ShapeNode::circle(0.5))
        .transform(Transform::Scale(Vec2::splat(0.5).into()))
        .transform(Transform::Translate(Vec2::new(0.5, 0.0).into())),
    )]);
    let lit = |count| line(&scene, count).iter().filter(|c| **c == RED).count();
    // the middle half, less the LEDs its edges pass through
    assert_eq!(lit(9), 3);
    assert_eq!(lit(101), 49);
  }

  #[test]
  fn nothing_is_seen_outside_the_viewport() {
    let scene = circle(solid(RED));
    let transparent = Rgba([0, 0, 0, 0]);
    assert_eq!(sample(&scene, -1.0, 25.0, 0.5), transparent);
    assert_eq!(sample(&scene, 101.0, 25.0, 0.5), transparent);
    assert_eq!(sample(&scene, 50.0, 51.0, 0.5), transparent);
  }

  #[test]
  fn a_shape_larger_than_the_viewport_can_move_under_it() {
    // a 300 wide gradient, moved so its middle is in view
    let scene = Scene::new(Viewport::Area, vec![Layer::new(red_to_blue()).shape(
      Shape::new(ShapeNode::Polygon {
        points: [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| Vec2::new(x, y).into()).to_vec(),
      })
      .transform(Transform::Scale(Vec2::new(300.0, 50.0).into()))
      .transform(Transform::Translate(Vec2::new(-100.0, 0.0).into())),
    )]);
    assert_eq!(sample(&scene, 50.0, 25.0, 0.0), Rgba([127, 0, 128, 255]));
  }

  #[test]
  fn a_grid_of_another_shape_does_not_squash_the_scene() {
    // 100 x 50 on a 128 x 32 grid: as wide as it is tall, with bars at the sides
    let scene = circle(solid(RED));
    let params = Params::new();
    let frame = Frame { time: 0.0, take: 0, params: &params, images: Images::none(), size: SIZE };
    let pixels: Vec<_> =
      Grid::new(128, 32, Vec2::new(100.0, 50.0)).samples().map(|s| scene.sample(&s, &frame)).collect();
    let row = |y: usize| pixels[y * 128..(y + 1) * 128].iter().filter(|c| **c == RED).count();
    let column = |x: usize| (0..32).filter(|y| pixels[y * 128 + x] == RED).count();
    assert!(row(16).abs_diff(column(64)) <= 2, "{} across, {} down", row(16), column(64));
    assert_eq!(pixels[0], Rgba([0, 0, 0, 0]));
  }

  /// A level that rises from 0 to 100 over a second, and stays there. Read back with `level_at`.
  fn rising(timing: Option<Timing>) -> Scene {
    let track = Prop::Track(Track {
      keys: vec![
        Key::new(0.0, Color(Rgba([0, 0, 0, 255])), Curve::Linear),
        Key::new(1.0, Color(Rgba([100, 0, 0, 255])), Curve::Linear),
      ],
      cycle: Cycle::Once,
    });
    let scene = Scene::new(Viewport::Sequence, vec![Layer::new(Fill::new(FillNode::Solid(track)))]);
    match timing {
      Some(timing) => scene.timing(timing),
      None => scene,
    }
  }

  fn level_at(scene: &Scene, time: f32) -> u8 {
    let params = Params::new();
    scene.sample(&Sample::new(Vec2::new(0.5, 0.0), 0.5), &Frame { time, take: 0, params: &params, images: Images::none(), size: SIZE })[0]
  }

  #[test]
  fn a_scene_that_plays_once_rests_at_its_end() {
    let scene = rising(Some(Timing::new(1.0, Cycle::Once)));
    assert_eq!(level_at(&scene, 0.5), 50);
    assert!(!scene.is_complete(0.5));
    assert_eq!(level_at(&scene, 1.0), 100);
    assert!(scene.is_complete(1.0));
    assert_eq!(level_at(&scene, 9.0), 100);
  }

  #[test]
  fn a_scene_repeats_as_many_times_as_it_says() {
    let scene = rising(Some(Timing::new(1.0, Cycle::Times(2))));
    assert_eq!(level_at(&scene, 1.5), 50);
    assert!(!scene.is_complete(1.5));
    assert!(scene.is_complete(2.0));
    assert_eq!(level_at(&scene, 2.5), 100);
  }

  #[test]
  fn a_scene_can_repeat_forever() {
    let scene = rising(Some(Timing::new(1.0, Cycle::Forever)));
    assert_eq!(level_at(&scene, 10.25), 25);
    assert!(!scene.is_complete(1000.0));
  }

  #[test]
  fn timing_cuts_a_track_that_runs_longer() {
    let scene = rising(Some(Timing::new(0.5, Cycle::Forever)));
    assert_eq!(level_at(&scene, 0.75), 25);
  }

  #[test]
  fn a_static_scene_has_no_time() {
    let scene = rising(None);
    assert_eq!(level_at(&scene, 0.5), 0);
    assert!(!scene.is_complete(1000.0));
  }

  #[test]
  fn keyframes_need_a_timing() {
    assert_eq!(rising(None).check(), Err(no_timing("layer 1")));
    assert_eq!(rising(Some(Timing::new(1.0, Cycle::Once))).check(), Ok(()));
    assert_eq!(Scene::new(Viewport::Sequence, vec![Layer::new(solid(RED))]).check(), Ok(()));
  }

  #[test]
  fn keyframes_are_found_wherever_a_scene_has_them() {
    fn track<T>() -> Track<T> {
      Track { keys: vec![], cycle: Cycle::Once }
    }
    let in_transform = Layer::new(solid(RED)).shape(Shape::new(ShapeNode::circle(0.5)).transform(Transform::Rotate(Prop::Track(track()))));
    let in_point = Layer::new(solid(RED)).shape(Shape::new(ShapeNode::Polygon { points: vec![Prop::Track(track())] }));
    let in_stop = Layer::new(Fill::new(FillNode::Gradient {
      stops: vec![Stop::new(0.0, Prop::Track(track()))],
      space: ColorSpace::Rgb,
      style: GradientStyle::Linear,
    }));
    let in_adjust = Layer::new(solid(RED).adjust(ColorAdjust::HueShift(Prop::Track(track()))));
    for layer in [in_transform, in_point, in_stop, in_adjust] {
      assert_eq!(Scene::new(Viewport::Area, vec![layer]).check(), Err(no_timing("layer 1")));
    }
  }

  #[test]
  fn a_timing_needs_a_length() {
    for duration in [0.0, -1.0, f32::NAN, f32::INFINITY] {
      let scene = Scene::new(Viewport::Area, vec![]).timing(Timing::new(duration, Cycle::Once));
      assert_eq!(scene.check(), Err(SceneError::NoDuration), "{duration}");
    }
  }

  #[test]
  fn params_override_defaults() {
    use crate::scene::Value;

    let mut scene = Scene::new(
      Viewport::Sequence,
      vec![Layer::new(Fill::new(FillNode::Solid(Prop::Param("color".into()))))],
    );
    scene.params.insert("color".into(), Value::Color(Color(RED)));
    assert_eq!(line(&scene, 1), vec![RED]);

    let params = Params::from([("color".into(), Value::Color(Color(BLUE)))]);
    let frame = Frame { time: 0.0, take: 0, params: &params, images: Images::none(), size: SIZE };
    assert_eq!(scene.sample(&Sample::new(Vec2::ZERO, 0.5), &frame), BLUE);
  }

  /// Red on the left half and blue on the right, with a green row across the middle
  fn image_fill() -> Fill {
    Fill::new(FillNode::image("art.png"))
  }

  fn images() -> Images {
    const GREEN: Rgba<u8> = Rgba([0, 255, 0, 255]);
    let art = image::RgbaImage::from_fn(4, 3, |x, y| match (x, y) {
      (_, 1) => GREEN,
      (0 | 1, _) => RED,
      _ => BLUE,
    });
    let mut images = Images::new();
    images.insert("art.png", Picture::new(art));
    images
  }

  fn drawn(scene: &Scene, x: f32, y: f32, radius: f32) -> Rgba<u8> {
    let (params, images) = (Params::new(), images());
    let frame = Frame { time: 0.0, take: 0, params: &params, images: &images, size: Vec2::new(4.0, 3.0) };
    scene.sample(&Sample::new(Vec2::new(x, y), radius), &frame)
  }

  #[test]
  fn an_image_shows_its_pixels_on_a_grid_of_the_same_size() {
    let scene = Scene::new(Viewport::Area, vec![Layer::new(image_fill())]);
    let (params, images) = (Params::new(), images());
    let size = Vec2::new(4.0, 3.0);
    let frame = Frame { time: 0.0, take: 0, params: &params, images: &images, size };
    let colors: Vec<_> = Grid::new(4, 3, size).samples().map(|at| scene.sample(&at, &frame)).collect();

    let green = Rgba([0, 255, 0, 255]);
    assert_eq!(colors, [RED, RED, BLUE, BLUE, green, green, green, green, RED, RED, BLUE, BLUE]);
  }

  #[test]
  fn an_image_stretches_over_a_shape_and_moves_with_it() {
    let shape = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(Vec2::splat(2.0).into())).transform(Transform::Translate(Vec2::new(2.0, 1.5).into()));
    let scene = Scene::new(Viewport::Area, vec![Layer::new(image_fill()).shape(shape)]);
    assert_eq!(drawn(&scene, 1.5, 0.7, 0.01), RED);
    assert_eq!(drawn(&scene, 2.5, 0.7, 0.01), BLUE);
    assert_eq!(drawn(&scene, 0.1, 0.1, 0.01)[3], 0);
  }

  #[test]
  fn a_strip_runs_along_the_middle_of_an_image() {
    let scene = Scene::new(Viewport::Sequence, vec![Layer::new(image_fill())]);
    let (params, images) = (Params::new(), images());
    let frame = Frame { time: 0.0, take: 0, params: &params, images: &images, size: SIZE };
    for at in Line::new(5).samples() {
      assert_eq!(scene.sample(&Sample::new(at.pos, 0.0), &frame), Rgba([0, 255, 0, 255]));
    }
  }

  #[test]
  fn a_shape_scaled_down_covers_more_of_its_image_with_each_sample() {
    let checkers = image::RgbaImage::from_fn(64, 64, |x, y| if (x + y) % 2 == 0 { RED } else { BLUE });
    let mut images = Images::new();
    images.insert("art.png", Picture::new(checkers));
    let (params, size) = (Params::new(), Vec2::new(64.0, 64.0));
    let frame = Frame { time: 0.0, take: 0, params: &params, images: &images, size };

    // a whole viewport of it shows each pixel, and the same image in a shape of 4 units has 16 pixels to each unit
    let at = Sample::new(Vec2::new(32.0, 32.0), 0.5);
    let whole = Scene::new(Viewport::Area, vec![Layer::new(image_fill())]);
    assert!([RED, BLUE].contains(&whole.sample(&at, &frame)));

    let shape = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(Vec2::splat(4.0).into())).transform(Transform::Translate(Vec2::splat(32.0).into()));
    let small = Scene::new(Viewport::Area, vec![Layer::new(image_fill()).shape(shape)]);
    let color = small.sample(&at, &frame);
    assert!((100..=155).contains(&color[0]) && (100..=155).contains(&color[2]), "{color:?}");
  }

  #[test]
  fn transforms_and_adjusts_work_on_an_image_like_any_fill() {
    let flipped = image_fill().transform(Transform::Rotate(180.0.into()));
    let scene = Scene::new(Viewport::Area, vec![Layer::new(flipped)]);
    assert_eq!(drawn(&scene, 0.5, 0.5, 0.01), BLUE);
    assert_eq!(drawn(&scene, 3.5, 0.5, 0.01), RED);

    let tinted = image_fill().adjust(ColorAdjust::HueShift(120.0.into()));
    let scene = Scene::new(Viewport::Area, vec![Layer::new(tinted)]);
    assert_ne!(drawn(&scene, 0.5, 0.5, 0.01), RED);
  }

  #[test]
  fn an_image_shrunk_by_a_transform_leaves_the_rest_clear() {
    let shrunk = image_fill().transform(Transform::Scale(Vec2::splat(0.5).into()));
    let scene = Scene::new(Viewport::Area, vec![Layer::new(shrunk), ]);
    assert_eq!(drawn(&scene, 0.2, 0.2, 0.01)[3], 0);
    assert_eq!(drawn(&scene, 1.5, 1.0, 0.01), RED);
  }

  #[test]
  fn a_scene_lists_the_images_it_draws() {
    let scene = Scene::new(Viewport::Area, vec![Layer::new(solid(RED)), Layer::new(image_fill())]);
    assert_eq!(scene.image_names().collect::<Vec<_>>(), ["art.png"]);
  }

  /// A scene of an animated image, one pixel for each frame number
  fn playing(frame: Prop<f32>) -> Scene {
    let image = FillNode::Image { name: "a.gif".into(), frame: Some(frame) };
    Scene::new(Viewport::Area, vec![Layer::new(Fill::new(image))]).timing(Timing::new(1.0, Cycle::Forever))
  }

  /// Which of 4 frames is drawn. Each frame's red is its number.
  fn frame_at(scene: &Scene, time: f32, take: u32, params: &Params) -> u8 {
    let art = (0..4).map(|n| image::RgbaImage::from_pixel(1, 1, Rgba([n, 0, 0, 255])));
    let mut images = Images::new();
    images.insert("a.gif", Picture::animated(art, 0.4));
    let frame = Frame { time, take, params, images: &images, size: Vec2::ONE };
    scene.sample(&Sample::new(Vec2::splat(0.5), 0.1), &frame)[0]
  }

  #[test]
  fn keys_going_from_0_to_the_frame_count_spend_the_same_time_on_each() {
    let track = Track { keys: vec![Key::new(0.0, 0.0, Curve::Linear), Key::new(1.0, 4.0, Curve::Linear)], cycle: Cycle::Once };
    let scene = playing(Prop::Track(track));
    let frames = [0.1, 0.3, 0.6, 0.9].map(|time| frame_at(&scene, time, 0, &Params::new()));
    assert_eq!(frames, [0, 1, 2, 3]);
  }

  #[test]
  fn a_param_chooses_a_frame() {
    let scene = playing(Prop::Param("frame".into()));
    let params = Params::from([("frame".into(), crate::scene::Value::Float(2.0))]);
    assert_eq!(frame_at(&scene, 0.0, 0, &params), 2);
  }

  #[test]
  fn a_spread_up_to_the_frame_count_picks_each_frame_about_as_often() {
    let random = Random { picks: vec![Pick::new(0.0)], spread: Spread::Float(0.0, 4.0), seed: 1, force_unique: true };
    let scene = playing(Prop::Random(random));
    let mut times = [0; 4];
    for take in 0..4000 {
      times[frame_at(&scene, 0.5, take, &Params::new()) as usize] += 1;
    }
    assert!(times.iter().all(|times| (900..1100).contains(times)), "{times:?}");
  }

  #[test]
  fn a_random_frame_is_a_different_frame_at_every_pick() {
    let random = Random { picks: vec![Pick::new(0.0)], spread: Spread::Float(0.0, 4.0), seed: 1, force_unique: true };
    let scene = playing(Prop::Random(random));
    let frames: Vec<u8> = (0..300).map(|cycle| frame_at(&scene, cycle as f32 + 0.5, 0, &Params::new())).collect();
    assert!(frames.windows(2).all(|pair| pair[0] != pair[1]), "{frames:?}");
  }

  #[test]
  fn a_frame_that_changes_needs_timing() {
    let track = Track { keys: vec![Key::new(0.0, 0.0, Curve::Linear)], cycle: Cycle::Once };
    let image = FillNode::Image { name: "a.gif".into(), frame: Some(Prop::Track(track)) };
    let scene = Scene::new(Viewport::Area, vec![Layer::new(Fill::new(image))]);
    assert!(matches!(scene.check(), Err(SceneError::NoTiming { .. })));
  }

  #[test]
  fn an_image_yet_to_be_chosen_is_not_listed() {
    let scene = Scene::new(Viewport::Area, vec![Layer::new(Fill::new(FillNode::image("")))]);
    assert_eq!(scene.image_names().count(), 0);
    assert_eq!(sample(&scene, 1.0, 1.0, 0.5)[3], 0);
  }

  #[test]
  fn an_image_with_nothing_to_draw_it_is_clear_and_needs_no_timing() {
    let scene = Scene::new(Viewport::Area, vec![Layer::new(image_fill())]);
    assert!(scene.check().is_ok());
    assert_eq!(sample(&scene, 1.0, 1.0, 0.5)[3], 0);
  }

  #[test]
  fn a_target_has_the_layers_that_play_on_it() {
    let scene = Scene::new(Viewport::Sequence, vec![
      Layer::new(solid(RED)),
      Layer::new(solid(BLUE)).targets(["center"]),
    ])
    .targets(["ring", "center"]);

    assert_eq!(line(&scene.for_target("ring"), 1)[0], RED);
    assert_eq!(line(&scene.for_target("center"), 1)[0], BLUE);
  }

  #[test]
  fn a_layer_can_play_on_several_targets() {
    let scene = Scene::new(Viewport::Area, vec![Layer::new(solid(RED)).targets(["a", "b"])]).targets(["a", "b", "c"]);
    let counts: Vec<_> = ["a", "b", "c"].iter().map(|target| scene.for_target(target).layers.len()).collect();
    assert_eq!(counts, [1, 1, 0]);
  }

  #[test]
  fn a_layer_must_name_one_of_the_scenes_targets() {
    let scene = Scene::new(Viewport::Sequence, vec![Layer::new(solid(RED)).targets(["ring"])]);
    assert!(matches!(scene.check(), Err(SceneError::UnknownTarget { .. })));
  }

  #[test]
  fn targets_have_their_own_names() {
    let scene = Scene::new(Viewport::Sequence, vec![]).targets(["ring", "ring"]);
    assert_eq!(scene.check(), Err(SceneError::RepeatedTarget { target: "ring".to_string() }));
  }
}
