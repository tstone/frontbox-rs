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
  /// Drawn bottom to top, alpha over
  #[serde(default)]
  pub layers: Vec<Layer>,
}

impl Scene {
  /// A scene of the current format version
  pub fn new(viewport: Viewport, layers: Vec<Layer>) -> Self {
    Self { version: file::VERSION, params: Params::new(), viewport, timing: None, layers }
  }

  pub fn timing(mut self, timing: Timing) -> Self {
    self.timing = Some(timing);
    self
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
    match self.timing {
      Some(timing) if !(timing.duration.is_finite() && timing.duration > 0.0) => Err(SceneError::NoDuration),
      None if self.layers.iter().any(Layer::is_animated) => Err(SceneError::NoTiming),
      _ => Ok(()),
    }
  }

  /// The names of the images it draws, which can repeat
  pub fn image_names(&self) -> impl Iterator<Item = &str> {
    self.layers.iter().filter_map(|layer| layer.fill.base.image_name())
  }

  /// The color at a sample. Transparent outside the viewport.
  pub fn sample(&self, at: &Sample, frame: &Frame) -> Rgba<u8> {
    let env = self.env(frame);
    let viewport = self.viewport.bounds(frame.size);
    if !viewport.contains(at.pos) {
      return Rgba([0, 0, 0, 0]);
    }
    self
      .layers
      .iter()
      .fold(Rgba([0, 0, 0, 0]), |below, layer| layer.sample(at, viewport, &env).composite_over(below))
  }

  /// Where each layer is drawn, as a closed loop of points in scene units. A layer with no shape outlines the viewport,
  /// and one that draws nothing has no points.
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
    }
  }
}

/// Why a scene can't be played
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneError {
  /// It has props that change over time, but no timing to play them in
  NoTiming,
  /// Its timing has no length
  NoDuration,
}

impl fmt::Display for SceneError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NoTiming => write!(f, "a scene that changes over time needs a timing"),
      Self::NoDuration => write!(f, "a scene's duration must be more than zero"),
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
    Color, ColorAdjust, ColorSpace, Fill, FillNode, Images, Key, Pick, Picture, Prop, Random, Shape, ShapeNode, Spread, Stop,
    Track, Transform,
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
    })
  }

  fn sample(scene: &Scene, x: f32, y: f32, radius: f32) -> Rgba<u8> {
    let params = Params::new();
    let frame = Frame { time: 0.0, take: 0, params: &params, images: Images::none(), size: SIZE };
    scene.sample(&Sample::new(Vec2::new(x, y), radius), &frame)
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
    assert_eq!(scene.check().unwrap_err(), SceneError::NoTiming);
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
  fn mirror_makes_a_strip_symmetric_from_the_middle_of_a_fill() {
    let fill = red_to_blue().transform(Transform::Mirror(0.0.into()));
    let colors = line(&Scene::new(Viewport::Sequence, vec![Layer::new(fill)]), 7);
    assert_eq!(colors[0], BLUE);
    assert_eq!(colors[6], BLUE);
    assert_eq!(colors[1], colors[5]);
    assert_eq!(colors[2], colors[4]);
  }

  #[test]
  fn repeat_tiles_a_fill_along_a_strip() {
    let fill = red_to_blue().transform(Transform::Scale(Vec2::new(0.5, 1.0).into())).transform(Transform::Repeat(Vec2::new(0.5, 0.0).into()));
    let scene = Scene::new(Viewport::Sequence, vec![Layer::new(fill)]);
    assert_ne!(sample(&scene, 0.3, 0.0, 0.1), sample(&scene, 0.4, 0.0, 0.1));
    // the copies meet at 0.25 and 0.75, so these stay within one
    for x in [0.3, 0.375, 0.45] {
      assert_eq!(sample(&scene, x, 0.0, 0.1), sample(&scene, x + 0.5, 0.0, 0.1));
    }
  }

  #[test]
  fn repeat_copies_a_shape_and_a_later_transform_moves_the_copies() {
    let shape = Shape::new(ShapeNode::circle(0.5))
      .transform(Transform::Scale(Vec2::splat(4.0).into()))
      .transform(Transform::Repeat(Vec2::new(20.0, 0.0).into()))
      .transform(Transform::Translate(Vec2::new(10.0, 25.0).into()));
    let scene = Scene::new(Viewport::Area, vec![Layer::new(solid(RED)).shape(shape)]);
    for x in [10.0, 30.0, 50.0, 90.0] {
      assert_eq!(sample(&scene, x, 25.0, 0.0), RED, "{x}");
      assert_eq!(sample(&scene, x + 10.0, 25.0, 0.0)[3], 0, "{x}");
    }
    assert_eq!(sample(&scene, 10.0, 35.0, 0.0)[3], 0);
  }

  #[test]
  fn a_mirrored_shape_keeps_its_soft_edge_on_both_sides() {
    let shape = Shape::new(ShapeNode::circle(0.5))
      .transform(Transform::Scale(Vec2::new(20.0, 10.0).into()))
      .transform(Transform::Translate(Vec2::new(30.0, 0.0).into()))
      .transform(Transform::Mirror(0.0.into()))
      .transform(Transform::Translate(Vec2::new(50.0, 25.0).into()));
    let scene = Scene::new(Viewport::Area, vec![Layer::new(solid(RED)).shape(shape)]);
    assert_eq!(sample(&scene, 80.0, 25.0, 1.0), RED);
    assert_eq!(sample(&scene, 20.0, 25.0, 1.0), RED);
    assert_eq!(sample(&scene, 50.0, 25.0, 1.0)[3], 0);
    assert_eq!(sample(&scene, 90.0, 25.0, 1.0), sample(&scene, 10.0, 25.0, 1.0));
    assert_eq!(sample(&scene, 90.0, 25.0, 1.0)[3], 128);
  }

  #[test]
  fn an_outline_shows_the_drawing_a_fold_starts_from() {
    let params = Params::new();
    let frame = Frame { time: 0.0, take: 0, params: &params, images: Images::none(), size: SIZE };
    let plain = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Translate(Vec2::new(10.0, 5.0).into()));
    let mut folded = plain.clone();
    folded.transform.insert(0, Transform::Repeat(Vec2::splat(4.0).into()));
    folded.transform.insert(1, Transform::Mirror(30.0.into()));
    let outlines = |shape| Scene::new(Viewport::Area, vec![Layer::new(solid(RED)).shape(shape)]).outlines(&frame);
    assert_eq!(outlines(folded), outlines(plain));
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
    assert_eq!(rising(None).check(), Err(SceneError::NoTiming));
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
    }));
    let in_adjust = Layer::new(solid(RED).adjust(ColorAdjust::HueShift(Prop::Track(track()))));
    for layer in [in_transform, in_point, in_stop, in_adjust] {
      assert_eq!(Scene::new(Viewport::Area, vec![layer]).check(), Err(SceneError::NoTiming));
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
    assert!(matches!(scene.check(), Err(SceneError::NoTiming)));
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
}
