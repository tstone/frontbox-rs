use std::fmt;
use std::sync::Arc;

use glam::Vec2;

use crate::scene::generator::{Kept, Moment, unknown_setting};
use crate::scene::{
  GeneratorError, GeneratorFrame, GeneratorHandle, GeneratorKind, GeneratorSetting, ShapeNode,
};

/// Outlines made by code outside gfx, for [`ShapeNode::External`](crate::scene::ShapeNode::External)
///
/// A generator is for shapes too involved to build from the base shapes, such as a skeleton posed by kinematics or a
/// tentacle that curls. A scene names it and sets its settings; whatever loads the scene puts it in the scene's
/// [`Assets`](crate::scene::Assets) by that name. Its output is a shape
/// like any other: transforms place it, and strokes, feathers, and shadows draw on it.
///
/// [`Self::generate`] has to be a pure function of the frame it's given, so that any moment can be sampled on its own,
/// seeking included, and its output can be kept for every sample of a frame.
pub trait ShapeGenerator: Send + Sync + fmt::Debug {
  /// What a scene can set, which the editor shows. A scene's setting by any other name is an error when it loads.
  fn settings(&self) -> &[GeneratorSetting];

  /// Whether its output changes with time when its settings don't. False keeps one output for the whole playback.
  fn is_animated(&self) -> bool {
    false
  }

  /// Whether its output has a [`Generated::path`], which a [`Sweep`](crate::scene::Sweep) can follow. A sweep can't
  /// follow a generator without one: a scene with one fails to load.
  fn has_path(&self) -> bool {
    false
  }

  /// Points of its settings that an editor can show a handle on, to drag. None unless it lists them.
  fn handles(&self) -> &[GeneratorHandle] {
    &[]
  }

  /// The outline at `frame`, in local space, before transforms
  fn generate(&self, frame: &GeneratorFrame) -> Generated;
}

/// What a [`ShapeGenerator`] makes: areas or lines, covered as one, and maybe a path to follow
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Generated {
  parts: Vec<Vec<Vec2>>,
  open: bool,
  path: Option<GeneratedPath>,
}

impl Generated {
  /// Closed polygons, each with an inside. Where they overlap they're one area, so a stroke traces only the outside of
  /// them all. A part of fewer than 3 points has no inside, and is left out.
  pub fn areas(parts: impl IntoIterator<Item = Vec<Vec2>>) -> Self {
    Self {
      parts: parts.into_iter().filter(|part| part.len() >= 3).collect(),
      open: false,
      path: None,
    }
  }

  /// Lines through points, first to last, with no inside: a stroke draws them, centered on them. A part with no points
  /// is left out.
  pub fn lines(parts: impl IntoIterator<Item = Vec<Vec2>>) -> Self {
    Self {
      parts: parts.into_iter().filter(|part| !part.is_empty()).collect(),
      open: true,
      path: None,
    }
  }

  /// Nothing, which draws nothing
  pub fn empty() -> Self {
    Self::default()
  }

  /// Adds the path a [`Sweep`](crate::scene::Sweep) follows, through `points`. A closed one goes around back to its
  /// first point, and a sweep wraps around it; an open one ends at its last, and a sweep holds there.
  pub fn path(mut self, points: Vec<Vec2>, closed: bool) -> Self {
    self.path = Some(GeneratedPath { points, closed });
    self
  }

  pub fn parts(&self) -> &[Vec<Vec2>] {
    &self.parts
  }

  /// Whether its parts are lines, with no inside
  pub fn is_open(&self) -> bool {
    self.open
  }

  pub fn followed(&self) -> Option<&GeneratedPath> {
    self.path.as_ref()
  }
}

/// The path through a [`Generated`] that a [`Sweep`](crate::scene::Sweep) follows
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedPath {
  pub points: Vec<Vec2>,
  pub closed: bool,
}

/// A [`ShapeGenerator`] in a scene's [`Assets`](crate::scene::Assets), which keeps its latest outputs so that every
/// sample of a frame, and every layer with the same settings, shares one call. Clones share what's kept.
#[derive(Debug, Clone)]
pub struct LoadedGenerator {
  generator: Arc<dyn ShapeGenerator>,
  kept: Kept<Generated>,
}

impl LoadedGenerator {
  pub fn new(generator: Arc<dyn ShapeGenerator>) -> Self {
    Self {
      generator,
      kept: Kept::default(),
    }
  }

  /// `generator` for the external shape `shape` names it in, which can only set the settings it has. `None` for a
  /// shape that isn't external.
  pub fn for_shape(
    generator: Arc<dyn ShapeGenerator>,
    shape: &ShapeNode,
  ) -> Option<Result<Self, GeneratorError>> {
    let name = shape.generator_name()?;
    let unknown = unknown_setting(generator.settings(), shape.generator_settings());
    Some(match unknown {
      Some(setting) => Err(GeneratorError::NoSuchSetting {
        kind: GeneratorKind::Shape,
        generator: name.into(),
        setting: setting.into(),
      }),
      None => Ok(Self::new(generator)),
    })
  }

  pub fn generator(&self) -> &dyn ShapeGenerator {
    self.generator.as_ref()
  }

  /// Where each of its [`ShapeGenerator::handles`] is, in its local space, with `setting` giving the scene's value for
  /// a setting it sets. A handle on a setting the generator doesn't list is at 0 there.
  pub(crate) fn handle_points(&self, setting: impl Fn(&str) -> Option<f32>) -> Vec<Vec2> {
    let specs = self.generator.settings();
    let value = |name: &str| {
      specs
        .iter()
        .find(|spec| spec.name == name)
        .map_or(0.0, |spec| {
          spec.clamped(setting(spec.name).unwrap_or(spec.default))
        })
    };
    self
      .generator
      .handles()
      .iter()
      .map(|handle| Vec2::new(value(handle.x), value(handle.y)))
      .collect()
  }

  /// The output at a moment, with `setting` giving the scene's value for a setting it sets
  pub(crate) fn generate(
    &self,
    time: f32,
    cycle: u32,
    seed: u32,
    setting: impl Fn(&str) -> Option<f32>,
  ) -> Arc<Generated> {
    let moment = Moment {
      time,
      cycle,
      seed,
      animated: self.generator.is_animated(),
    };
    self
      .kept
      .output(self.generator.settings(), moment, setting, |frame| {
        self.generator.generate(frame)
      })
  }
}

#[cfg(test)]
mod tests {
  use std::sync::atomic::{AtomicUsize, Ordering};

  use super::*;

  #[derive(Debug, Default)]
  struct Counting {
    calls: AtomicUsize,
  }

  const SETTINGS: &[GeneratorSetting] = &[GeneratorSetting::new("size", 2.0).range(0.0, 10.0)];

  impl ShapeGenerator for Counting {
    fn settings(&self) -> &[GeneratorSetting] {
      SETTINGS
    }

    fn generate(&self, frame: &GeneratorFrame) -> Generated {
      self.calls.fetch_add(1, Ordering::Relaxed);
      let size = frame.setting("size");
      Generated::areas([vec![Vec2::ZERO, Vec2::new(size, 0.0), Vec2::new(0.0, size)]])
    }
  }

  #[test]
  fn an_output_is_kept_for_the_same_moment_and_settings() {
    let counting = Arc::new(Counting::default());
    let loaded = LoadedGenerator::new(counting.clone());
    loaded.generate(1.0, 0, 0, |_| None);
    loaded.generate(1.0, 0, 0, |_| None);
    assert_eq!(counting.calls.load(Ordering::Relaxed), 1);
    loaded.generate(1.0, 0, 0, |_| Some(3.0));
    assert_eq!(counting.calls.load(Ordering::Relaxed), 2);
  }

  #[test]
  fn a_generator_that_isnt_animated_makes_one_output_for_every_moment() {
    let counting = Arc::new(Counting::default());
    let loaded = LoadedGenerator::new(counting.clone());
    loaded.generate(1.0, 0, 0, |_| None);
    loaded.generate(2.0, 3, 0, |_| None);
    assert_eq!(counting.calls.load(Ordering::Relaxed), 1);
  }

  #[test]
  fn a_setting_is_its_default_when_left_out_and_clamped_to_its_range() {
    let loaded = LoadedGenerator::new(Arc::new(Counting::default()));
    let corner = |setting: Option<f32>| loaded.generate(0.0, 0, 0, |_| setting).parts()[0][1].x;
    assert_eq!(corner(None), 2.0);
    assert_eq!(corner(Some(50.0)), 10.0);
  }

  #[test]
  fn parts_too_small_to_draw_are_left_out() {
    assert!(
      Generated::areas([vec![Vec2::ZERO, Vec2::X]])
        .parts()
        .is_empty()
    );
    assert!(Generated::lines([vec![]]).parts().is_empty());
  }

  mod in_a_shape {
    use std::collections::BTreeMap;

    use image::Rgba;

    use super::*;
    use crate::scene::value::tests::env;
    use crate::scene::{
      Assets, Color, Env, Fill, FillNode, GeneratorError, Inputs, Layer, Prop, Regions, Sample,
      Shape, ShapeAdjust, ShapeNode, StrokeAlign, Sweep, Transform, Variables,
      check_generated_paths, file,
    };

    /// Unit squares in a row, `spacing` apart, centered on y = 0 from x = 0, with a path through their middles
    #[derive(Debug)]
    struct Squares;

    const SQUARES: &[GeneratorSetting] = &[
      GeneratorSetting::new("count", 2.0).range(0.0, 8.0),
      GeneratorSetting::new("spacing", 0.5),
    ];

    const SQUARES_HANDLES: &[GeneratorHandle] =
      &[GeneratorHandle::new("reach", "count", "spacing")];

    impl ShapeGenerator for Squares {
      fn settings(&self) -> &[GeneratorSetting] {
        SQUARES
      }

      fn has_path(&self) -> bool {
        true
      }

      fn handles(&self) -> &[GeneratorHandle] {
        SQUARES_HANDLES
      }

      fn generate(&self, frame: &GeneratorFrame) -> Generated {
        let spacing = frame.setting("spacing");
        let middles: Vec<Vec2> = (0..frame.setting("count") as u32)
          .map(|i| Vec2::new(i as f32 * spacing, 0.0))
          .collect();
        let square = |middle: &Vec2| {
          [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)]
            .map(|(x, y)| *middle + Vec2::new(x, y))
            .to_vec()
        };
        Generated::areas(middles.iter().map(square)).path(middles, false)
      }
    }

    /// One line from (0, 0) to (10, 0), with no path
    #[derive(Debug)]
    struct Line;

    impl ShapeGenerator for Line {
      fn settings(&self) -> &[GeneratorSetting] {
        &[]
      }

      fn generate(&self, _: &GeneratorFrame) -> Generated {
        Generated::lines([vec![Vec2::ZERO, Vec2::new(10.0, 0.0)]])
      }
    }

    fn assets() -> Assets {
      let mut assets = Assets::new();
      assets.insert_shape_generator("squares", LoadedGenerator::new(Arc::new(Squares)));
      assets.insert_shape_generator("line", LoadedGenerator::new(Arc::new(Line)));
      assets
    }

    fn external(generator: &str, settings: &[(&str, Prop<f32>)]) -> Shape {
      Shape::new(ShapeNode::External {
        generator: generator.into(),
        settings: settings
          .iter()
          .map(|(name, setting)| (name.to_string(), setting.clone()))
          .collect::<BTreeMap<_, _>>(),
      })
    }

    fn stroke(width: f32) -> ShapeAdjust {
      ShapeAdjust::Stroke {
        width: width.into(),
        align: StrokeAlign::Centered,
        fill: Fill::new(FillNode::Solid(Color(Rgba([255, 0, 0, 255])).into())),
      }
    }

    /// How much the shape covers at (x, y), and how much its first stroke does
    fn covered(shape: &Shape, assets: &Assets, x: f32, y: f32) -> (f32, f32) {
      let variables = Variables::new();
      let env = Env {
        assets,
        ..env(0.0, &variables)
      };
      let cover = shape
        .prepare(&env)
        .and_then(|shape| shape.cover(&Sample::new(Vec2::new(x, y), 0.01)));
      cover.map_or((0.0, 0.0), |cover| {
        let stroke = cover.strokes.first().map_or(0.0, |(amount, _)| *amount);
        (cover.amount, stroke)
      })
    }

    #[test]
    fn its_areas_are_covered_as_one() {
      let shape = external("squares", &[]);
      assert_eq!(covered(&shape, &assets(), -0.25, 0.0).0, 1.0);
      assert_eq!(covered(&shape, &assets(), 0.75, 0.0).0, 1.0);
      assert_eq!(covered(&shape, &assets(), 1.5, 0.0).0, 0.0);
    }

    #[test]
    fn a_stroke_traces_only_the_outside_of_its_areas() {
      let shape = external("squares", &[]).adjust(stroke(0.1));
      // the first square's right edge, inside the second
      assert_eq!(covered(&shape, &assets(), 0.5, 0.0).1, 0.0);
      // the second square's right edge, outside them both
      assert_eq!(covered(&shape, &assets(), 1.0, 0.0).1, 1.0);
    }

    #[test]
    fn its_pose_has_where_each_of_its_handles_is() {
      let variables = Variables::new();
      let assets = assets();
      let env = Env {
        assets: &assets,
        ..env(0.0, &variables)
      };
      // a setting the scene sets, held to its range, and one left at its default
      let shape = external("squares", &[("count", 20.0.into())]);
      assert_eq!(shape.pose(&env).unwrap().points, vec![Vec2::new(8.0, 0.5)]);
      assert!(external("line", &[]).pose(&env).unwrap().points.is_empty());
    }

    #[test]
    fn its_settings_are_props() {
      let apart = external(
        "squares",
        &[
          ("count", 3.0.into()),
          ("spacing", Prop::Variable("gap".into())),
        ],
      );
      let mut variables = Variables::new();
      variables.insert("gap".into(), crate::scene::Value::Float(2.0));
      let env = Env {
        assets: &assets(),
        ..env(0.0, &variables)
      };
      let prepared = apart.prepare(&env).unwrap();
      let at = |x: f32| {
        prepared
          .cover(&Sample::new(Vec2::new(x, 0.0), 0.01))
          .map_or(0.0, |c| c.amount)
      };
      assert_eq!(at(4.0), 1.0);
      assert_eq!(at(1.0), 0.0);
      let mut inputs = Inputs::new();
      apart.read_inputs(&mut inputs);
      assert!(inputs.variables.contains_key("gap"));
    }

    #[test]
    fn transforms_place_it() {
      let moved =
        external("squares", &[]).transform(Transform::Translate(Vec2::new(10.0, 0.0).into()));
      assert_eq!(covered(&moved, &assets(), 10.0, 0.0).0, 1.0);
      assert_eq!(covered(&moved, &assets(), 0.0, 0.0).0, 0.0);
    }

    #[test]
    fn its_lines_have_no_inside_and_a_stroke_draws_them() {
      let shape = external("line", &[]).adjust(stroke(0.5));
      assert_eq!(covered(&shape, &assets(), 5.0, 0.0), (0.0, 1.0));
    }

    #[test]
    fn a_generator_it_doesnt_have_draws_nothing() {
      let shape = external("squares", &[]).adjust(stroke(0.5));
      assert_eq!(covered(&shape, Assets::none(), 0.0, 0.0), (0.0, 0.0));
    }

    #[derive(Debug)]
    struct Track(Shape);

    impl Regions for Track {
      fn region(&self, layer: &str) -> Option<&Shape> {
        (layer == "track").then_some(&self.0)
      }
    }

    #[test]
    fn a_sweep_follows_its_path_and_holds_at_an_open_ones_ends() {
      let track = Track(external("squares", &[("count", 3.0.into())]));
      let variables = Variables::new();
      let env = Env {
        assets: &assets(),
        regions: &track,
        ..env(0.0, &variables)
      };
      let at =
        |position: f32| Prop::<Vec2>::Sweep(Sweep::new("track", position.into())).resolve(&env);
      assert_eq!(at(0.5), Vec2::new(0.5, 0.0));
      assert_eq!(at(1.5), Vec2::new(1.0, 0.0));
    }

    fn layer(name: &str, region: Shape) -> Layer {
      Layer::new(Fill::new(FillNode::Solid(
        Color(Rgba([255, 0, 0, 255])).into(),
      )))
      .name(name)
      .region(region)
    }

    fn follower() -> Layer {
      layer(
        "follower",
        Shape::new(ShapeNode::circle(1.0)).transform(Transform::Translate(Prop::Sweep(
          Sweep::new("track", 0.5.into()),
        ))),
      )
    }

    #[test]
    fn a_sweep_cant_follow_a_generator_with_no_path() {
      let check = |generator: &str| {
        let layers = [layer("track", external(generator, &[])), follower()];
        let layers: Vec<_> = layers.iter().enumerate().collect();
        check_generated_paths(&layers, &assets()).map_err(|error| error.error)
      };
      assert!(check("squares").is_ok());
      assert!(matches!(
        check("line"),
        Err(GeneratorError::NoPath { path, generator }) if path == "track" && generator == "line"
      ));
    }

    #[test]
    fn a_shape_can_only_set_the_settings_its_generator_has() {
      let loaded = |shape: Shape| LoadedGenerator::for_shape(Arc::new(Squares), &shape.base);
      assert!(
        loaded(external("squares", &[("count", 1.0.into())])).is_some_and(|loaded| loaded.is_ok())
      );
      assert!(matches!(
        loaded(external("squares", &[("size", 1.0.into())])),
        Some(Err(GeneratorError::NoSuchSetting { generator, setting, .. })) if generator == "squares" && setting == "size"
      ));
      assert!(loaded(Shape::new(ShapeNode::circle(1.0))).is_none());
    }

    #[test]
    fn it_round_trips_through_ron_with_no_settings_left_out() {
      for shape in [
        external("squares", &[]),
        external(
          "squares",
          &[
            ("count", 3.0.into()),
            ("spacing", Prop::Variable("gap".into())),
          ],
        ),
      ] {
        let written = file::save(&shape).unwrap();
        let read: Shape = file::load(&written).unwrap();
        assert_eq!(
          written.contains("settings"),
          shape.base.generator_settings().next().is_some(),
          "{written}"
        );
        for x in [-0.25, 0.75, 1.25, 1.75] {
          assert_eq!(
            covered(&read, &assets(), x, 0.0),
            covered(&shape, &assets(), x, 0.0),
            "{written}"
          );
        }
      }
    }
  }
}
