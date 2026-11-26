use std::fmt;

use glam::Vec2;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};

use crate::scene::{
  Animatable, Assets, ContentError, Env, Inputs, Layer, LayerAssetError, Prop, SceneError, Shape,
};

/// A position on the outline of another layer's region
///
/// The path is the region of the layer named `layer`, with its own transforms, so a sweep along a circle translated to a
/// point orbits it, and any number of layers can share one path. A layer that's only there to be a path can have a line
/// for its region, which draws nothing without a stroke, or not be visible: a sweep follows a region whether or not its
/// layer draws.
///
/// Built-in shapes go clockwise from where their outline starts (3 o'clock for round ones); shapes made of points follow
/// their point order. Circles and ellipses go at a constant angular speed, everything else at a constant speed.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sweep {
  /// The layer whose region it follows, by name. That has to be the only drawn layer by that name, with a region, and
  /// drawn on the same thing as the layer the sweep is in.
  pub layer: String,
  /// How far along the path, 0 to 1. Keyframes from 0 to 1 go along it once. Past 0 or 1 is up to `ends`.
  pub position: Box<Prop<f32>>,
  /// What a position past 0 or 1 does. Left out, it wraps around a closed shape and holds at the ends of a line.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub ends: Option<SweepEnds>,
}

impl Sweep {
  pub fn new(layer: impl Into<String>, position: Prop<f32>) -> Self {
    Self {
      layer: layer.into(),
      position: Box::new(position),
      ends: None,
    }
  }

  pub fn ends(mut self, ends: SweepEnds) -> Self {
    self.ends = Some(ends);
    self
  }

  /// The origin if there's no region by its layer's name, or if regions have followed each other too many times over
  /// to be going anywhere
  pub(crate) fn resolve(&self, env: &Env) -> Vec2 {
    let Some((path, path_env)) = env.region(&self.layer) else {
      return Vec2::ZERO;
    };
    let ends = self.ends.unwrap_or(if path.base.is_open_path(&path_env) {
      SweepEnds::Hold
    } else {
      SweepEnds::Wrap
    });
    path.along(ends.apply(self.position.resolve(env)), &path_env)
  }

  /// Only its position: the region it follows is its own layer's, which says for itself whether it moves
  pub(crate) fn is_animated(&self) -> bool {
    self.position.is_animated()
  }

  /// Its position's variables, and the layer it follows. What that layer's region reads is read by that layer.
  pub(crate) fn read_inputs(&self, inputs: &mut Inputs) {
    self.position.read_inputs(inputs);
    inputs.region(&self.layer);
  }
}

/// What a [`Sweep`]'s position does past 0 or 1
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SweepEnds {
  /// Stays at the start before 0 and at the end after 1
  Hold,
  /// Goes around again, so 2 is twice along it. A whole number past 0 is at the end, so a line keyframed to 1 stays at
  /// its end, and from just past it back at the start.
  Wrap,
  /// Comes back the way it went: out from 0 to 1, back from 1 to 2, and so on
  PingPong,
}

impl SweepEnds {
  /// Where `position` is along the path, from 0 to 1
  fn apply(self, position: f32) -> f32 {
    match self {
      Self::Hold => position.clamp(0.0, 1.0),
      Self::Wrap => {
        let along = position.rem_euclid(1.0);
        if along == 0.0 && position > 0.0 {
          1.0
        } else {
          along
        }
      }
      Self::PingPong => {
        let along = position.rem_euclid(2.0);
        if along > 1.0 { 2.0 - along } else { along }
      }
    }
  }
}

/// The regions of a scene's layers by name, for a [`Sweep`] to follow
pub trait Regions: fmt::Debug {
  /// The region of the layer named `layer`. `None` if there's no such layer, or it has no region.
  fn region(&self, layer: &str) -> Option<&Shape>;
}

/// No regions, for a layer sampled on its own
pub fn no_regions() -> &'static dyn Regions {
  &NoRegions
}

/// Checks each sweep in `layers` follows a region it can: one of `layers`, the only one with that name among them, with a
/// region, and drawn on the same thing as it, which `K` says. A layer whose region follows its way back to its own
/// region is an error too, as it would have no end.
///
/// Each layer comes with its index among the scene's layers, to name it by in errors.
pub fn check_sweeps<K: PartialEq>(layers: &[(usize, &Layer, K)]) -> Result<(), SceneError> {
  // for each layer, the layers its region follows, by where they are in `layers`
  let mut follows = vec![Vec::new(); layers.len()];
  for (at, (index, layer, on)) in layers.iter().enumerate() {
    let mut inputs = Inputs::new();
    layer.read_inputs(&mut inputs);
    let mut in_region = Inputs::new();
    layer
      .region
      .iter()
      .for_each(|region| region.read_path_inputs(&mut in_region));
    for name in &inputs.regions {
      let mut found = layers
        .iter()
        .enumerate()
        .filter(|(_, (_, layer, _))| layer.name.as_ref() == Some(name));
      let Some((path_at, (_, path, path_on))) = found.next() else {
        return Err(SceneError::NoPathToFollow {
          layer: layer.label(*index),
          path: name.clone(),
        });
      };
      if found.next().is_some() {
        return Err(SceneError::SamePathName { path: name.clone() });
      }
      if path.region.is_none() {
        return Err(SceneError::PathHasNoRegion { path: name.clone() });
      }
      if path_on != on {
        return Err(SceneError::PathElsewhere {
          layer: layer.label(*index),
          path: name.clone(),
        });
      }
      if in_region.regions.contains(name) {
        follows[at].push(path_at);
      }
    }
  }
  match looped(&follows) {
    Some(at) => Err(SceneError::PathLoop {
      layer: layers[at].1.label(layers[at].0),
    }),
    None => Ok(()),
  }
}

/// Checks no sweep in `layers` follows a region made by a shape generator with no path, which only `assets` knows. The
/// rest of what a sweep needs is [`check_sweeps`]'s.
///
/// Each layer comes with its index among the scene's layers, to name it by in errors.
pub fn check_generated_paths(
  layers: &[(usize, &Layer)],
  assets: &Assets,
) -> Result<(), LayerAssetError> {
  for (index, layer) in layers {
    let mut inputs = Inputs::new();
    layer.read_inputs(&mut inputs);
    for name in &inputs.regions {
      let generated = layers
        .iter()
        .filter(|(_, path)| path.name.as_ref() == Some(name))
        .filter_map(|(_, path)| path.region.as_ref()?.base.generator_name());
      for generator in generated {
        if assets
          .shape_generator(generator)
          .is_some_and(|loaded| !loaded.generator().has_path())
        {
          return Err(LayerAssetError {
            layer: layer.label(*index),
            error: ContentError::GeneratorHasNoPath {
              path: name.clone(),
              generator: generator.into(),
            },
          });
        }
      }
    }
  }
  Ok(())
}

/// Deserializes `Prop::Sweep`, rejecting it unless `T` is a position type
pub(crate) fn only_for_positions<'de, T: Animatable, D: Deserializer<'de>>(
  deserializer: D,
) -> Result<Sweep, D::Error> {
  if T::from_point(Vec2::ZERO).is_none() {
    return Err(D::Error::custom("only a position (Vec2) can be a Sweep"));
  }
  Sweep::deserialize(deserializer)
}

/// A layer in a loop of regions that follow each other, given what each one's region follows
fn looped(follows: &[Vec<usize>]) -> Option<usize> {
  // 0 is unvisited, 1 is on the way down from where the search started, 2 is done
  let mut state = vec![0u8; follows.len()];
  fn visit(at: usize, follows: &[Vec<usize>], state: &mut [u8]) -> bool {
    state[at] = 1;
    for &next in &follows[at] {
      if state[next] == 1 || (state[next] == 0 && visit(next, follows, state)) {
        return true;
      }
    }
    state[at] = 2;
    false
  }
  (0..follows.len()).find(|&at| state[at] == 0 && visit(at, follows, &mut state))
}

#[derive(Debug)]
struct NoRegions;

impl Regions for NoRegions {
  fn region(&self, _: &str) -> Option<&Shape> {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::animation::{Curve, Cycle};
  use crate::layout::Line;
  use crate::scene::test_scene::Scene;
  use crate::scene::value::tests::env;
  use crate::scene::{
    Assets, Color, Fill, FillNode, Key, Keyframes, LayerFrame, ShapeNode, Surface, Timing,
    Transform, Value, Variables, file, no_modulators, no_values, no_variables,
  };

  /// Paths by name
  #[derive(Debug)]
  struct Tracks(Vec<(&'static str, Shape)>);

  impl Regions for Tracks {
    fn region(&self, layer: &str) -> Option<&Shape> {
      self
        .0
        .iter()
        .find(|(name, _)| *name == layer)
        .map(|(_, shape)| shape)
    }
  }

  /// One track, called "track"
  fn track(path: Shape) -> Tracks {
    Tracks(vec![("track", path)])
  }

  /// `path` translated to (50, 20)
  fn placed(path: ShapeNode) -> Tracks {
    track(Shape::new(path).transform(Transform::Translate(Vec2::new(50.0, 20.0).into())))
  }

  /// Once along the path between `start` and `end` seconds
  fn lap(start: f32, end: f32) -> Prop<f32> {
    Prop::Keyframes(Keyframes::new(
      vec![
        Key::new(start, 0.0, Curve::Linear),
        Key::new(end, 1.0, Curve::Linear),
      ],
      Cycle::Once,
    ))
  }

  fn along(position: Prop<f32>) -> Prop<Vec2> {
    Prop::Sweep(Sweep::new("track", position))
  }

  fn at(prop: &Prop<Vec2>, time: f32, tracks: &Tracks) -> Vec2 {
    at_with(prop, time, tracks, &Variables::new())
  }

  fn at_with(prop: &Prop<Vec2>, time: f32, tracks: &Tracks, variables: &Variables) -> Vec2 {
    prop.resolve(&Env {
      regions: tracks,
      ..env(time, variables)
    })
  }

  fn line() -> Tracks {
    track(Shape::new(ShapeNode::Polyline {
      points: vec![Vec2::ZERO.into(), Vec2::new(10.0, 0.0).into()],
    }))
  }

  fn layer() -> Layer {
    Layer::new(Fill::new(FillNode::Solid(
      Color(image::Rgba([255, 0, 0, 255])).into(),
    )))
  }

  /// A layer named `name` with a circle region, moved along `follows` if there is one
  fn path_layer(name: &str, follows: Option<&str>) -> Layer {
    let mut region = Shape::new(ShapeNode::circle(1.0));
    if let Some(follows) = follows {
      region = region.transform(Transform::Translate(Prop::Sweep(Sweep::new(
        follows,
        0.5.into(),
      ))));
    }
    layer().name(name).region(region)
  }

  fn check(layers: &[Layer]) -> Result<(), SceneError> {
    let layers: Vec<_> = layers
      .iter()
      .enumerate()
      .map(|(index, layer)| (index, layer, ()))
      .collect();
    check_sweeps(&layers)
  }

  #[test]
  fn a_circle_orbits_its_center() {
    let tracks = placed(ShapeNode::circle(10.0));
    let orbit = along(lap(0.0, 4.0));
    assert!(at(&orbit, 0.0, &tracks).abs_diff_eq(Vec2::new(60.0, 20.0), 1e-4));
    assert!(at(&orbit, 1.0, &tracks).abs_diff_eq(Vec2::new(50.0, 30.0), 1e-4));
    assert!(at(&orbit, 2.0, &tracks).abs_diff_eq(Vec2::new(40.0, 20.0), 1e-4));
  }

  #[test]
  fn it_waits_for_its_start_and_stays_at_its_end() {
    let tracks = placed(ShapeNode::circle(10.0));
    let orbit = along(lap(1.0, 3.0));
    assert!(at(&orbit, 0.0, &tracks).abs_diff_eq(Vec2::new(60.0, 20.0), 1e-4));
    assert!(at(&orbit, 2.0, &tracks).abs_diff_eq(Vec2::new(40.0, 20.0), 1e-4));
    assert!(at(&orbit, 9.0, &tracks).abs_diff_eq(Vec2::new(60.0, 20.0), 1e-4));
  }

  #[test]
  fn a_closed_path_goes_around_again_past_one() {
    let tracks = track(Shape::new(ShapeNode::circle(10.0)));
    let twice = Keyframes::new(
      vec![
        Key::new(0.0, 0.0, Curve::Linear),
        Key::new(4.0, 2.0, Curve::Linear),
      ],
      Cycle::Once,
    );
    let twice = along(Prop::Keyframes(twice));
    assert!(at(&twice, 0.5, &tracks).abs_diff_eq(Vec2::new(0.0, 10.0), 1e-4));
    assert!(at(&twice, 2.5, &tracks).abs_diff_eq(Vec2::new(0.0, 10.0), 1e-4));
    assert!(at(&twice, 4.0, &tracks).abs_diff_eq(Vec2::new(10.0, 0.0), 1e-4));
  }

  #[test]
  fn a_line_holds_at_its_ends() {
    let tracks = line();
    let at_position = |position: f32| at(&along(position.into()), 0.0, &tracks);
    assert_eq!(at_position(-0.5), Vec2::ZERO);
    assert_eq!(at_position(1.5), Vec2::new(10.0, 0.0));
  }

  #[test]
  fn a_spiral_holds_at_its_ends_as_a_line_does() {
    let spiral = ShapeNode::Spiral {
      radius: 10.0.into(),
      turns: 2.0.into(),
      flare: 0.0.into(),
    };
    let tracks = track(Shape::new(spiral));
    let end = at(&along(1.0.into()), 0.0, &tracks);
    assert!(at(&along(1.5.into()), 0.0, &tracks).abs_diff_eq(end, 1e-4));
  }

  #[test]
  fn a_line_can_ping_pong() {
    let tracks = line();
    let at_position = |position: f32| {
      let prop = Prop::Sweep(Sweep::new("track", position.into()).ends(SweepEnds::PingPong));
      at(&prop, 0.0, &tracks)
    };
    assert!(at_position(0.25).abs_diff_eq(Vec2::new(2.5, 0.0), 1e-4));
    assert!(at_position(1.0).abs_diff_eq(Vec2::new(10.0, 0.0), 1e-4));
    assert!(at_position(1.25).abs_diff_eq(Vec2::new(7.5, 0.0), 1e-4));
    assert!(at_position(2.0).abs_diff_eq(Vec2::ZERO, 1e-4));
    assert!(at_position(2.25).abs_diff_eq(Vec2::new(2.5, 0.0), 1e-4));
    assert!(at_position(-0.25).abs_diff_eq(Vec2::new(2.5, 0.0), 1e-4));
  }

  #[test]
  fn a_circle_can_ping_pong_back_the_way_it_came() {
    let tracks = track(Shape::new(ShapeNode::circle(10.0)));
    let prop =
      |position: f32| Prop::Sweep(Sweep::new("track", position.into()).ends(SweepEnds::PingPong));
    assert!(at(&prop(0.25), 0.0, &tracks).abs_diff_eq(Vec2::new(0.0, 10.0), 1e-4));
    assert!(at(&prop(1.75), 0.0, &tracks).abs_diff_eq(Vec2::new(0.0, 10.0), 1e-4));
  }

  #[test]
  fn a_line_that_wraps_ends_at_its_end_and_starts_again_past_it() {
    let tracks = line();
    let at_position = |position: f32| {
      let prop = Prop::Sweep(Sweep::new("track", position.into()).ends(SweepEnds::Wrap));
      at(&prop, 0.0, &tracks)
    };
    assert!(at_position(1.0).abs_diff_eq(Vec2::new(10.0, 0.0), 1e-4));
    assert!(at_position(1.25).abs_diff_eq(Vec2::new(2.5, 0.0), 1e-4));
    assert!(at_position(0.0).abs_diff_eq(Vec2::ZERO, 1e-4));
  }

  #[test]
  fn a_closed_path_can_hold() {
    let tracks = track(Shape::new(ShapeNode::circle(10.0)));
    let prop = Prop::Sweep(Sweep::new("track", 1.25.into()).ends(SweepEnds::Hold));
    assert!(at(&prop, 0.0, &tracks).abs_diff_eq(Vec2::new(10.0, 0.0), 1e-4));
  }

  #[test]
  fn its_position_can_come_from_a_variable_without_a_timing() {
    let tracks = track(Shape::new(ShapeNode::circle(10.0)));
    let sweep = along(Prop::Variable("lap".into()));
    let variables = Variables::from([("lap".into(), Value::Float(0.5))]);
    assert!(at_with(&sweep, 0.0, &tracks, &variables).abs_diff_eq(Vec2::new(-10.0, 0.0), 1e-4));
    assert!(!sweep.is_animated());
  }

  #[test]
  fn a_rect_goes_around_at_an_even_speed() {
    let tracks = placed(ShapeNode::Rect {
      width: 30.0.into(),
      height: 10.0.into(),
    });
    let around = along(lap(0.0, 8.0));
    // 80 around, starting from the top left corner, so 30 along the top is the top right corner
    assert!(at(&around, 0.0, &tracks).abs_diff_eq(Vec2::new(35.0, 15.0), 1e-4));
    assert!(at(&around, 3.0, &tracks).abs_diff_eq(Vec2::new(65.0, 15.0), 1e-4));
    assert!(at(&around, 3.5, &tracks).abs_diff_eq(Vec2::new(65.0, 20.0), 1e-4));
    assert!(at(&around, 8.0, &tracks).abs_diff_eq(Vec2::new(35.0, 15.0), 1e-4));
  }

  #[test]
  fn a_polygon_goes_through_its_points_in_order() {
    let points = [
      Vec2::ZERO,
      Vec2::new(10.0, 0.0),
      Vec2::new(10.0, 10.0),
      Vec2::new(0.0, 10.0),
    ];
    let tracks = placed(ShapeNode::Polygon {
      points: points.map(Into::into).to_vec(),
    });
    let around = along(lap(0.0, 4.0));
    assert!(at(&around, 1.0, &tracks).abs_diff_eq(Vec2::new(60.0, 20.0), 1e-4));
    assert!(at(&around, 2.5, &tracks).abs_diff_eq(Vec2::new(55.0, 30.0), 1e-4));
  }

  #[test]
  fn with_no_layer_by_its_name_it_stays_at_the_origin() {
    let tracks = placed(ShapeNode::circle(10.0));
    let lost = Prop::Sweep(Sweep::new("elsewhere", 0.25.into()));
    assert_eq!(at(&lost, 0.0, &tracks), Vec2::ZERO);
  }

  #[test]
  fn a_path_with_no_edge_stays_at_the_origin() {
    let tracks = track(Shape::new(ShapeNode::Polygon { points: vec![] }));
    assert_eq!(at(&along(0.5.into()), 0.5, &tracks), Vec2::ZERO);
  }

  #[test]
  fn a_path_can_be_on_a_sweep_of_its_own() {
    let orbit = Shape::new(ShapeNode::circle(10.0))
      .transform(Transform::Translate(Vec2::new(50.0, 20.0).into()));
    let moon = Shape::new(ShapeNode::circle(1.0)).transform(Transform::Translate(Prop::Sweep(
      Sweep::new("orbit", lap(0.0, 4.0)),
    )));
    let tracks = Tracks(vec![("orbit", orbit), ("moon", moon)]);
    let on_moon = Prop::Sweep(Sweep::new("moon", 0.0.into()));
    assert!(at(&on_moon, 0.0, &tracks).abs_diff_eq(Vec2::new(61.0, 20.0), 1e-4));
    assert!(at(&on_moon, 1.0, &tracks).abs_diff_eq(Vec2::new(51.0, 30.0), 1e-4));
  }

  #[test]
  fn regions_that_follow_each_other_stop() {
    let follows = |other: &'static str| {
      Shape::new(ShapeNode::circle(1.0)).transform(Transform::Translate(Prop::Sweep(Sweep::new(
        other,
        0.0.into(),
      ))))
    };
    let tracks = Tracks(vec![("a", follows("b")), ("b", follows("a"))]);
    let on_a = Prop::Sweep(Sweep::new("a", 0.0.into()));
    assert!(at(&on_a, 0.0, &tracks).is_finite());
  }

  #[test]
  fn a_sequence_sees_a_path_as_it_passes_along_the_line() {
    let path = Shape::new(ShapeNode::circle(0.25))
      .transform(Transform::Translate(Vec2::new(0.5, 0.0).into()));
    let moved = Transform::Translate(along(lap(0.0, 2.0)));
    let tall = ShapeNode::Rect {
      width: 0.1.into(),
      height: 2.0.into(),
    };
    let scene = Scene::new(
      Surface::Sequence,
      vec![
        // a path with no inside or stroke, so it draws nothing
        layer().name("track").region(path).visible(false),
        layer().region(Shape::new(tall).transform(moved)),
      ],
    );
    let scene = scene.timing(Timing::new(2.0, Cycle::Forever));
    scene.check().unwrap();

    let lit = |time: f32| {
      let variables = Variables::new();
      let frame = LayerFrame {
        time,
        playback_seed: 0,
        variables: &variables,
        assets: Assets::none(),
        size: Vec2::ZERO,
        surface: Surface::Area,
        visits: None,
        timing: None,
        defaults: no_variables(),
        values: no_values(),
        modulators: no_modulators(),
        regions: no_regions(),
      };
      let line = Line::new(5);
      line
        .samples()
        .map(|sample| scene.sample(&sample, &frame)[0] > 0)
        .collect::<Vec<_>>()
    };
    assert_eq!(lit(0.0), [false, false, false, true, false]);
    assert_eq!(lit(1.0), [false, true, false, false, false]);
  }

  #[test]
  fn a_shapes_pose_has_the_paths_it_moves_along() {
    let tracks = placed(ShapeNode::circle(10.0));
    let shape = Shape::new(ShapeNode::circle(1.0))
      .transform(Transform::Scale(1.0.into()))
      .transform(Transform::Translate(along(lap(0.0, 4.0))));
    let variables = Variables::new();
    let env = Env {
      regions: &tracks,
      ..env(0.0, &variables)
    };
    let pose = shape.pose(&env).unwrap();
    assert!(pose.paths[0].is_none());
    let path = pose.paths[1].as_ref().unwrap();
    assert_eq!(path.bounds.max, Vec2::splat(10.0));
    assert!(path.outline[0].abs_diff_eq(Vec2::new(60.0, 20.0), 1e-4));
  }

  #[test]
  fn it_reads_the_layer_it_follows() {
    let mut inputs = Inputs::new();
    Sweep::new("track", Prop::Variable("lap".into())).read_inputs(&mut inputs);
    assert!(inputs.regions.contains("track"));
    assert!(inputs.variables.contains_key("lap"));
  }

  #[test]
  fn it_survives_a_round_trip() {
    let sweep: Prop<Vec2> =
      Prop::Sweep(Sweep::new("track", Prop::Variable("lap".into())).ends(SweepEnds::PingPong));
    let written = file::save(&sweep).unwrap();
    assert_eq!(
      written,
      r#"Sweep(
  layer: "track",
  position: Variable("lap"),
  ends: PingPong,
)"#
    );
    let read: Prop<Vec2> = file::load(&written).unwrap();
    let tracks = line();
    let variables = Variables::from([("lap".into(), Value::Float(1.25))]);
    assert_eq!(
      at_with(&read, 0.0, &tracks, &variables),
      at_with(&sweep, 0.0, &tracks, &variables)
    );
  }

  #[test]
  fn a_sweep_has_to_follow_a_layer_there_is() {
    let error = check(&[path_layer("moon", Some("orbit"))]).unwrap_err();
    assert_eq!(
      error,
      SceneError::NoPathToFollow {
        layer: "layer \"moon\"".into(),
        path: "orbit".into()
      }
    );
  }

  #[test]
  fn a_sweep_has_to_follow_one_layer_by_its_name() {
    let error = check(&[
      path_layer("orbit", None),
      path_layer("orbit", None),
      path_layer("moon", Some("orbit")),
    ])
    .unwrap_err();
    assert_eq!(
      error,
      SceneError::SamePathName {
        path: "orbit".into()
      }
    );
  }

  #[test]
  fn a_sweep_has_to_follow_a_layer_with_a_region() {
    let error = check(&[layer().name("orbit"), path_layer("moon", Some("orbit"))]).unwrap_err();
    assert_eq!(
      error,
      SceneError::PathHasNoRegion {
        path: "orbit".into()
      }
    );
  }

  #[test]
  fn a_sweep_has_to_follow_a_layer_on_the_same_thing() {
    let orbit = path_layer("orbit", None);
    let moon = path_layer("moon", Some("orbit"));
    let error = check_sweeps(&[(0, &orbit, "dmd"), (1, &moon, "playfield")]).unwrap_err();
    assert_eq!(
      error,
      SceneError::PathElsewhere {
        layer: "layer \"moon\"".into(),
        path: "orbit".into()
      }
    );
  }

  #[test]
  fn regions_cant_follow_their_way_back_to_themselves() {
    let error = check(&[
      path_layer("a", Some("b")),
      path_layer("b", Some("c")),
      path_layer("c", Some("a")),
    ])
    .unwrap_err();
    assert!(matches!(error, SceneError::PathLoop { .. }));
    let error = check(&[path_layer("a", Some("a"))]).unwrap_err();
    assert!(matches!(error, SceneError::PathLoop { .. }));
  }

  #[test]
  fn a_fill_can_follow_its_own_layers_region() {
    let mut orbit = path_layer("orbit", None);
    orbit.fill = orbit
      .fill
      .transform(Transform::Translate(Prop::Sweep(Sweep::new(
        "orbit",
        0.0.into(),
      ))));
    check(&[orbit]).unwrap();
  }

  #[test]
  fn paths_that_follow_others_without_a_loop_are_fine() {
    check(&[
      path_layer("sun", None),
      path_layer("earth", Some("sun")),
      path_layer("moon", Some("earth")),
      path_layer("ship", Some("moon")),
    ])
    .unwrap();
  }
}
