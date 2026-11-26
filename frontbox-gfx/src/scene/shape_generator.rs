use std::fmt;
use std::sync::{Arc, Mutex};

use glam::Vec2;

/// Outlines made by code outside gfx, for [`ShapeNode::External`](crate::scene::ShapeNode::External)
///
/// A generator is for shapes too involved to build from the base shapes, such as a skeleton posed by kinematics or a
/// tentacle that curls. A scene names it and sets its settings; the app gives it to
/// [`Content::with_shape_generators`](crate::scene::Content::with_shape_generators) by that name. Its output is a shape
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

  /// The outline at `frame`, in local space, before transforms
  fn generate(&self, frame: &GeneratorFrame) -> Generated;
}

/// One setting of a [`ShapeGenerator`], which a scene sets as a `Prop<f32>`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneratorSetting {
  pub name: &'static str,
  /// What it is when a scene leaves it out
  pub default: f32,
  /// The lowest and highest it can be. A scene's value outside it is clamped.
  pub range: Option<(f32, f32)>,
}

impl GeneratorSetting {
  pub const fn new(name: &'static str, default: f32) -> Self {
    Self {
      name,
      default,
      range: None,
    }
  }

  pub const fn range(mut self, min: f32, max: f32) -> Self {
    self.range = Some((min, max));
    self
  }

  fn clamped(&self, value: f32) -> f32 {
    match self.range {
      Some((min, max)) => value.clamp(min, max),
      None => value,
    }
  }
}

/// The moment a [`ShapeGenerator`] makes its outline for, and its settings there
#[derive(Debug, Clone, Copy)]
pub struct GeneratorFrame<'a> {
  /// Seconds into the current scene cycle. 0 for a generator that isn't animated.
  pub time: f32,
  /// Index of the current scene cycle, from 0. 0 for a generator that isn't animated.
  pub cycle: u32,
  /// Changes each time a scene restarts, for a generator that picks at random
  pub seed: u32,
  specs: &'a [GeneratorSetting],
  values: &'a [f32],
}

impl GeneratorFrame<'_> {
  /// The setting named `name`, within its range. 0 for a name the generator doesn't list.
  pub fn setting(&self, name: &str) -> f32 {
    let at = self.specs.iter().position(|spec| spec.name == name);
    debug_assert!(at.is_some(), "no generator setting \"{name}\"");
    at.map_or(0.0, |at| self.values[at])
  }
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
  kept: Arc<Mutex<Vec<Kept>>>,
}

impl LoadedGenerator {
  /// Outputs kept: enough for the layers of a scene sharing one generator with different settings
  const KEPT: usize = 16;

  pub fn new(generator: Arc<dyn ShapeGenerator>) -> Self {
    Self {
      generator,
      kept: Arc::default(),
    }
  }

  pub fn generator(&self) -> &dyn ShapeGenerator {
    self.generator.as_ref()
  }

  /// The output at a moment, with `setting` giving the scene's value for a setting it sets
  pub(crate) fn generate(
    &self,
    time: f32,
    cycle: u32,
    seed: u32,
    setting: impl Fn(&str) -> Option<f32>,
  ) -> Arc<Generated> {
    let specs = self.generator.settings();
    let values: Vec<f32> = specs
      .iter()
      .map(|spec| spec.clamped(setting(spec.name).unwrap_or(spec.default)))
      .collect();
    let (time, cycle) = if self.generator.is_animated() {
      (time, cycle)
    } else {
      (0.0, 0)
    };
    let key = GeneratedKey {
      time: time.to_bits(),
      cycle,
      seed,
      values: values.iter().map(|value| value.to_bits()).collect(),
    };
    if let Some((_, generated)) = self
      .kept
      .lock()
      .unwrap()
      .iter()
      .find(|(kept, _)| *kept == key)
    {
      return generated.clone();
    }
    let generated = Arc::new(self.generator.generate(&GeneratorFrame {
      time,
      cycle,
      seed,
      specs,
      values: &values,
    }));
    let mut kept = self.kept.lock().unwrap();
    if kept.len() >= Self::KEPT {
      kept.remove(0);
    }
    kept.push((key, generated.clone()));
    generated
  }
}

/// An output a [`LoadedGenerator`] keeps, by what it was made for
type Kept = (GeneratedKey, Arc<Generated>);

/// What a [`LoadedGenerator`]'s output is kept by: the moment and resolved settings it was made for, as bits so they
/// compare exactly
#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedKey {
  time: u32,
  cycle: u32,
  seed: u32,
  values: Vec<u32>,
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
      Assets, Color, Content, ContentError, Env, Fill, FillNode, Inputs, Layer, Prop, Regions,
      Sample, Shape, ShapeAdjust, ShapeNode, StrokeAlign, Sweep, Transform, Variables,
      check_generated_paths, file,
    };

    /// Unit squares in a row, `spacing` apart, centered on y = 0 from x = 0, with a path through their middles
    #[derive(Debug)]
    struct Squares;

    const SQUARES: &[GeneratorSetting] = &[
      GeneratorSetting::new("count", 2.0).range(0.0, 8.0),
      GeneratorSetting::new("spacing", 0.5),
    ];

    impl ShapeGenerator for Squares {
      fn settings(&self) -> &[GeneratorSetting] {
        SQUARES
      }

      fn has_path(&self) -> bool {
        true
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
        Err(ContentError::GeneratorHasNoPath { path, generator }) if path == "track" && generator == "line"
      ));
    }

    #[test]
    fn content_gives_a_layer_its_generator_and_only_the_settings_it_has() {
      let content = Content::without_folder()
        .with_shape_generators([("squares", Arc::new(Squares) as Arc<dyn ShapeGenerator>)]);
      assert_eq!(content.shape_generator_names(), ["squares"]);
      let assets = content
        .assets_for([&layer("a", external("squares", &[("count", 1.0.into())]))])
        .unwrap();
      assert!(assets.shape_generator("squares").is_some());
      let error = |shape: Shape| content.assets_for([&layer("a", shape)]).unwrap_err().error;
      assert!(matches!(
        error(external("tentacle", &[])),
        ContentError::NoShapeGenerator(name) if name == "tentacle"
      ));
      assert!(matches!(
        error(external("squares", &[("size", 1.0.into())])),
        ContentError::NoSuchSetting { setting, .. } if setting == "size"
      ));
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
