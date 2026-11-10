//! Reading and writing scene files as RON
//!
//! Files are written with RON's `implicit_some` and `unwrap_variant_newtypes` extensions, so they only state what
//! they change and avoid double parentheses. Fields that aren't written take their default.
//!
//! The top-level type, such as `LedScene` or `DmdScene`, is defined by the crate that owns it. Each is wrapped in a
//! variant named for its kind, so a file says what it is and one kind can't be loaded as another.

use std::fmt;

use ron::extensions::Extensions;
use ron::ser::PrettyConfig;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// The format version this build writes, and the newest it reads
pub const VERSION: u32 = 1;

/// Scene file extension, in lowercase
pub const EXTENSION: &str = "scene";

/// Parses a scene file without validating it. Call [`GfxComposition::check`](crate::scene::GfxComposition::check) on
/// the result.
pub fn load<T: DeserializeOwned>(source: &str) -> Result<T, FormatError> {
  options().from_str(source).map_err(FormatError::Read)
}

/// Serializes a scene to RON text
pub fn save<T: Serialize>(scene: &T) -> Result<String, FormatError> {
  let config = PrettyConfig::new().indentor("  ").extensions(extensions());
  options().to_string_pretty(scene, config).map_err(FormatError::Write)
}

/// Errors if `version` is newer than this build can read
pub fn check_version(version: u32) -> Result<(), FormatError> {
  if version > VERSION {
    return Err(FormatError::Newer { version });
  }
  Ok(())
}

fn extensions() -> Extensions {
  Extensions::IMPLICIT_SOME | Extensions::UNWRAP_VARIANT_NEWTYPES
}

fn options() -> ron::Options {
  ron::Options::default().with_default_extension(extensions())
}

#[derive(Debug)]
pub enum FormatError {
  Read(ron::error::SpannedError),
  Write(ron::Error),
  /// The file was written by a newer version of the format
  Newer { version: u32 },
  /// Parsed, but failed validation
  Invalid(String),
}

impl fmt::Display for FormatError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Read(e) => write!(f, "couldn't read scene: {e}"),
      Self::Write(e) => write!(f, "couldn't write scene: {e}"),
      Self::Newer { version } => {
        write!(f, "scene is format version {version}, but this build reads up to {VERSION}")
      }
      Self::Invalid(e) => write!(f, "{e}"),
    }
  }
}

impl std::error::Error for FormatError {}

#[cfg(test)]
mod tests {
  use glam::Vec2;
  use image::Rgba;

  use super::*;
  use crate::animation::{Curve, Cycle};
  use crate::layout::Line;
  use crate::scene::test_scene::Scene;
  use crate::scene::*;

  fn load(source: &str) -> Result<Scene, FormatError> {
    let composition: GfxComposition = super::load(source)?;
    composition.check().map_err(|err| FormatError::Invalid(err.to_string()))?;
    Ok(Scene::with_composition(Surface::Area, composition))
  }

  fn save(scene: &Scene) -> Result<String, FormatError> {
    super::save(&scene.composition)
  }

  fn sample_grid(scene: &Scene) -> Vec<Rgba<u8>> {
    let params = Params::new();
    [0.0, 0.5, 1.0, 1.5]
      .into_iter()
      .flat_map(|time| {
        let frame = FrameContext { time, playback_seed: 0, params: &params, images: Images::none(), size: Vec2::new(100.0, 50.0), surface: scene.surface, visits: None };
        (0..20)
          .flat_map(|y| (0..40).map(move |x| Sample::new(Vec2::new(x as f32 * 2.5, y as f32 * 2.5), 1.0)))
          .map(move |s| scene.sample(&s, &frame))
      })
      .collect()
  }

  /// Uses every type in a definition
  fn everything() -> Scene {
    let mut scene = Scene::new(Surface::Area, vec![
        Layer::new(Fill::new(FillNode::Solid(Prop::Param("background".into())))),
        Layer::new(
          Fill::new(FillNode::Gradient {
            stops: vec![
              Stop::new(0.0, Color(Rgba([255, 204, 0, 255]))),
              Stop::new(1.0, Color(Rgba([255, 51, 0, 128]))),
            ],
            style: GradientStyle::Radial,
          })
          .transform(Transform::Rotate(90.0.into()))
          .adjust(ColorAdjust::HueShift(Prop::Param("hue".into())))
          .adjust(ColorAdjust::Opacity(0.75.into()))
          .adjust(ColorAdjust::Brightness(1.25.into()))
          .adjust(ColorAdjust::Contrast(1.5.into())),
        )
        .region(
          Shape::new(ShapeNode::circle(0.5))
            .transform(Transform::Scale(Vec2::splat(40.0).into()))
            .transform(Transform::Translate(Prop::Track(Track {
              keys: vec![
                Key::new(0.0, Vec2::new(30.0, 25.0), Curve::Linear),
                Key::new(1.0, Vec2::new(70.0, 25.0), Curve::EaseInOut),
              ],
              cycle: Cycle::Forever,
            }))),
        ),
        Layer::new(Fill::new(FillNode::Solid(Color(Rgba([0, 0, 255, 255])).into()))).region(Shape::new(
          ShapeNode::Polygon {
            points: vec![Vec2::new(0.0, 0.0).into(), Vec2::new(20.0, 0.0).into(), Vec2::new(10.0, 15.0).into()],
          },
        )),
        Layer::new(Fill::new(FillNode::Solid(Color(Rgba([255, 0, 0, 255])).into()))).region(Shape::new(
          ShapeNode::Ring { inner_radius: 3.0.into(), thickness: 1.0.into() },
        )),
        Layer::new(Fill::new(FillNode::Solid(Color(Rgba([255, 255, 0, 255])).into())))
          .hidden()
          .region(Shape::new(ShapeNode::Pinwheel { blades: 4.0.into(), blade_fraction: 0.25.into() })),
        Layer::new(Fill::new(FillNode::Solid(Color(Rgba([0, 255, 0, 255])).into()))).region(
          Shape::new(ShapeNode::circle(4.0)).transform(Transform::Translate(Prop::Sweep(Sweep::new(
            Shape::new(ShapeNode::circle(15.0)).transform(Transform::Translate(Vec2::new(50.0, 25.0).into())),
            0.25,
            1.0,
          )))),
        ),
        Layer::new(Fill::new(FillNode::Solid(Color(Rgba([0, 255, 255, 255])).into()))).region(
          Shape::new(ShapeNode::circle(4.0)).transform(Transform::Translate(Prop::Sweep(Sweep::new(
            Shape::new(ShapeNode::Polyline { points: vec![Vec2::ZERO.into(), Vec2::new(10.0, 0.0).into()] }),
            0.0,
            1.0,
          )))),
        ),
        Layer::new(Fill::new(FillNode::Solid(Color(Rgba([255, 0, 255, 255])).into()))).region(
          Shape::new(ShapeNode::circle(4.0)).transform(Transform::Translate(Prop::Sweep(Sweep::new(
            Shape::new(ShapeNode::Bezier {
              points: vec![
                BezierPoint { point: Vec2::ZERO.into(), handle: Vec2::new(0.0, 5.0).into() },
                BezierPoint { point: Vec2::new(10.0, 0.0).into(), handle: Vec2::new(0.0, -5.0).into() },
              ],
            }),
            0.0,
            1.0,
          )))),
        ),
    ]);
    scene.params.insert("background".into(), Value::Color(Color(Rgba([16, 16, 16, 255]))));
    scene.timing(Timing::new(1.0, Cycle::Forever))
  }

  #[test]
  fn a_sweep_only_loads_on_a_position() {
    let sweep = "Sweep(path: (base: Ellipse(rx: Fixed(1.0), ry: Fixed(1.0))), start: 0.0, end: 1.0)";
    let scene = |transform: &str, radius: &str, color: &str| {
      format!(
        "(timing: (duration: 1.0), layers: [(
          region: (transform: [{transform}], base: Ellipse(rx: Fixed(1.0), ry: {radius})),
          fill: (base: Solid({color})),
        )])"
      )
    };
    let (radius, color) = ("Fixed(1.0)", "Fixed(255, 0, 0, 255)");
    assert!(load(&scene(&format!("Translate({sweep})"), radius, color)).is_ok());

    let not_positions = [
      scene(&format!("Rotate({sweep})"), radius, color),
      scene("", sweep, color),
      scene("", radius, sweep),
    ];
    for source in not_positions {
      let error = load(&source).unwrap_err().to_string();
      assert!(error.contains("only a position"), "{error}");
    }
  }

  #[test]
  fn a_random_color_with_no_alpha_is_opaque() {
    let scene = load("(timing: (duration: 1.0), layers: [(fill: (base: Solid(Random(
      picks: [(at: 0.0)],
      spread: Color(hue: (0.0, 360.0), brightness: (1.0, 1.0)),
    ))))])");
    let scene = scene.unwrap();
    let FillNode::Solid(Prop::Random(random)) = &scene.layers[0].fill.base else {
      panic!("{scene:?}");
    };
    assert!(matches!(random.spread, Spread::Color { alpha: (1.0, 1.0), .. }));
  }

  #[test]
  fn a_random_forces_unique_picks_unless_told_otherwise() {
    let scene = load("(timing: (duration: 1.0), layers: [(fill: (base: Solid(Random(
      picks: [(at: 0.0)],
      spread: Color(hue: (0.0, 360.0), brightness: (1.0, 1.0)),
    ))))])");
    let scene = scene.unwrap();
    let FillNode::Solid(Prop::Random(random)) = &scene.layers[0].fill.base else {
      panic!("{scene:?}");
    };
    assert!(random.force_unique);
  }

  #[test]
  fn random_props_read_and_write() {
    let source = "(timing: (duration: 2.0), layers: [(fill: (base: Solid(Random(
      picks: [(at: 0.0), (at: 1.0, smooth_in: 0.25, smooth_out: 0.5)],
      spread: Color(hue: (330.0, 390.0), brightness: (0.5, 1.0), alpha: (0.25, 1.0)),
      seed: 7,
      force_unique: false,
    ))))])";
    let written = save(&load(source).unwrap()).unwrap();
    let loaded = load(&written).unwrap();
    let FillNode::Solid(Prop::Random(random)) = &loaded.layers[0].fill.base else {
      panic!("{written}");
    };
    assert_eq!(random.picks, [Pick::new(0.0), Pick { at: 1.0, smooth_in: 0.25, smooth_out: 0.5 }]);
    assert_eq!(random.spread, Spread::Color { hue: (330.0, 390.0), brightness: (0.5, 1.0), alpha: (0.25, 1.0) });
    assert_eq!(random.seed, 7);
    assert!(!random.force_unique);
  }

  #[test]
  fn round_trip_samples_the_same() {
    let scene = everything();
    let written = save(&scene).unwrap();
    let read = load(&written).unwrap();
    assert_eq!(sample_grid(&read), sample_grid(&scene), "written as:\n{written}");
    assert_eq!(save(&read).unwrap(), written);
  }

  #[test]
  fn reads_a_hand_written_file() {
    let mut scene = load(
      r#"(
        layers: [
          (fill: (base: Gradient(stops: [
            (at: 0.0, color: Fixed(255, 0, 0, 255)),
            (at: 1.0, color: Fixed(0, 0, 255, 255)),
          ]))),
        ],
      )"#,
    )
    .unwrap();
    scene.surface = Surface::Sequence;

    let params = Params::new();
    let line = Line::new(3);
    let frame = FrameContext { time: 0.0, playback_seed: 0, params: &params, images: Images::none(), size: Vec2::new(100.0, 50.0), surface: scene.surface, visits: None };
    let colors: Vec<_> = line.samples().map(|s| scene.sample(&s, &frame)).collect();
    assert_eq!(colors[0], Rgba([255, 0, 0, 255]));
    assert_eq!(colors[2], Rgba([0, 0, 255, 255]));
  }

  #[test]
  fn a_gradient_with_no_style_is_linear() {
    let scene = load("(layers: [(fill: (base: Gradient(stops: [])))])").unwrap();
    assert!(matches!(scene.layers[0].fill.base, FillNode::Gradient { style: GradientStyle::Linear, .. }));
  }

  #[test]
  fn a_gradient_from_before_colors_only_mixed_in_rgb_still_reads() {
    let scene = load(
      "(layers: [(fill: (base: Gradient(stops: [
        (at: 0.0, color: Fixed(255, 0, 0, 255)),
      ], space: Oklab, style: Radial)))])",
    )
    .unwrap();
    assert!(matches!(scene.layers[0].fill.base, FillNode::Gradient { style: GradientStyle::Radial, .. }));
  }

  #[test]
  fn shapes_from_before_their_fields_were_renamed_still_read() {
    let scene = load(
      "(layers: [
        (fill: (base: Solid(Fixed(255, 0, 0, 255))), shape: (base: Ellipse(r1: Fixed(2.0), r2: Fixed(10.0)))),
        (fill: (base: Solid(Fixed(255, 0, 0, 255))), shape: (base: Ring(radius: Fixed(3.0), thickness: Fixed(1.0)))),
        (fill: (base: Solid(Fixed(255, 0, 0, 255))), shape: (base: Pinwheel(blades: Fixed(4.0), balance: Fixed(0.25)))),
        (fill: (base: Solid(Fixed(255, 0, 0, 255))), shape: (base: Route(points: []))),
      ])",
    )
    .unwrap();
    let base = |index: usize| &scene.layers[index].region.as_ref().unwrap().base;
    assert!(matches!(base(0), ShapeNode::Ellipse { rx: Prop::Fixed(10.0), ry: Prop::Fixed(2.0) }));
    assert!(matches!(base(1), ShapeNode::Ring { inner_radius: Prop::Fixed(3.0), .. }));
    assert!(matches!(base(2), ShapeNode::Pinwheel { blade_fraction: Prop::Fixed(0.25), .. }));
    assert!(matches!(base(3), ShapeNode::Polyline { .. }));

    let written = save(&scene).unwrap();
    assert!(["rx:", "inner_radius:", "blade_fraction:", "Polyline("].iter().all(|name| written.contains(name)), "{written}");
  }

  #[test]
  fn an_animated_image_keeps_its_frame_when_written_and_read() {
    let frame = Track {
      keys: vec![Key::new(0.0, 0.0, Curve::Linear), Key::new(1.0, 2.0, Curve::Linear)],
      cycle: Default::default(),
    };
    let image = FillNode::Image { name: "a.gif".into(), frame: Some(Prop::Track(frame)) };
    let scene = Scene::new(Surface::Area, vec![Layer::new(Fill::new(image))]).timing(Timing::new(1.0, Cycle::Once));
    let read = load(&save(&scene).unwrap()).unwrap();

    let first = Rgba([255, 0, 0, 255]);
    let last = Rgba([0, 0, 255, 255]);
    let art = [first, first, last].map(|color| image::RgbaImage::from_pixel(1, 1, color));
    let (mut images, params) = (Images::new(), Params::new());
    images.insert("a.gif", Picture::animated(art, 0.3));

    let size = Vec2::new(1.0, 1.0);
    let at = Sample::new(Vec2::splat(0.5), 0.1);
    let color = |scene: &Scene, time| scene.sample(&at, &FrameContext { time, playback_seed: 0, params: &params, images: &images, size, surface: scene.surface, visits: None });
    for time in [0.0, 0.4, 1.0] {
      assert_eq!(color(&read, time), color(&scene, time));
    }
    assert_eq!(color(&read, 0.0), first);
    assert_eq!(color(&read, 1.0), last);
  }

  #[test]
  fn an_image_is_written_and_read_by_its_name() {
    let scene = Scene::new(Surface::Area, vec![Layer::new(Fill::new(FillNode::image("dmd/title.png")))]);
    let written = save(&scene).unwrap();
    assert!(written.contains(r#"name: "dmd/title.png""#) && !written.contains("frame"), "{written}");

    let read = load(&written).unwrap();
    assert_eq!(read.image_names().collect::<Vec<_>>(), ["dmd/title.png"]);

    let art = image::RgbaImage::from_fn(2, 2, |x, _| if x == 0 { Rgba([255, 0, 0, 255]) } else { Rgba([0, 0, 255, 255]) });
    let (mut images, params) = (Images::new(), Params::new());
    images.insert("dmd/title.png", Picture::new(art));
    let size = Vec2::new(2.0, 2.0);
    let frame = FrameContext { time: 0.0, playback_seed: 0, params: &params, images: &images, size, surface: scene.surface, visits: None };
    let at = Sample::new(Vec2::new(1.5, 0.5), 0.5);
    assert_eq!(read.sample(&at, &frame), scene.sample(&at, &frame));
    assert_eq!(read.sample(&at, &frame), Rgba([0, 0, 255, 255]));
  }

  #[test]
  fn reads_a_timing() {
    let scene = load("(timing: (duration: 2.5, cycle: Times(3)), layers: [])").unwrap();
    assert_eq!(scene.timing, Some(Timing::new(2.5, Cycle::Times(3))));

    // playing once is the default
    let scene = load("(timing: (duration: 2.5), layers: [])").unwrap();
    assert_eq!(scene.timing, Some(Timing::new(2.5, Cycle::Once)));
  }

  #[test]
  fn a_static_scene_writes_no_timing() {
    let written = save(&Scene::new(Surface::Area, vec![])).unwrap();
    assert!(!written.contains("timing"), "{written}");
    assert_eq!(load(&written).unwrap().timing, None);
  }

  #[test]
  fn refuses_keyframes_with_no_timing() {
    let result = load(
      r#"(
        layers: [(fill: (base: Solid(Track(keys: [(at: 0.0, value: (0, 0, 0, 255))]))))],
      )"#,
    );
    assert!(matches!(result, Err(FormatError::Invalid(_))), "{result:?}");
  }

  #[test]
  fn a_layer_keeps_its_name_and_only_writes_one_it_has() {
    let fill = || Fill::new(FillNode::Solid(Color(Rgba([255, 0, 0, 255])).into()));
    let scene = Scene::new(Surface::Area, vec![Layer::new(fill()).name("sky"), Layer::new(fill())]);

    let written = save(&scene).unwrap();
    assert_eq!(written.matches("name:").count(), 1, "{written}");
    let read = load(&written).unwrap();
    assert_eq!(read.layers[0].name.as_deref(), Some("sky"));
    assert_eq!(read.layers[1].name, None);
  }

  #[test]
  fn refuses_a_timing_with_no_length() {
    let result = load("(timing: (duration: 0.0), layers: [])");
    assert!(matches!(result, Err(FormatError::Invalid(_))), "{result:?}");
  }

  #[test]
  fn a_layer_is_only_hidden_if_it_says_so() {
    let scene = load("(layers: [(fill: (base: Gradient(stops: [])))])").unwrap();
    assert!(!scene.layers[0].hidden);
    assert!(!save(&scene).unwrap().contains("hidden"));
  }

  #[test]
  fn a_mask_survives_a_save() {
    let fill = || Fill::new(FillNode::Solid(Color(Rgba([0, 0, 0, 255])).into()));
    let scene = Scene::new(Surface::Area, vec![Layer::new(fill()).mask(Mask::new(fill()).inverted())]);
    let read = load(&save(&scene).unwrap()).unwrap();
    assert!(read.layers[0].mask.as_ref().is_some_and(|mask| mask.invert && mask.region.is_none()));
  }

  /// A layer on a graph using every algorithm, with responses to two of them
  fn on_a_graph() -> Scene {
    let traversal = |name: &str, algorithm| Traversal::new(name, NodeRef::name("0"), algorithm, 4.0);
    let traversals = vec![
      traversal("breadth", TraversalAlgorithm::BreadthFirst { wander: 0.0.into() }).start(0.5),
      traversal("depth", TraversalAlgorithm::DepthFirst),
      traversal("distance", TraversalAlgorithm::Dijkstra { wander: 0.5.into() }),
      traversal("path", TraversalAlgorithm::ShortestPath { to: NodeRef::name("8"), wander: 0.5.into() }),
      Traversal::new("search", NodeRef::Nearest(Prop::Param("hit".into())), TraversalAlgorithm::AStar { to: NodeRef::name("0") }, Prop::Param("speed".into())),
      Traversal::new("walk", NodeRef::name("0"), TraversalAlgorithm::RandomWalk, Speed::Duration(None)),
    ];
    let shown = VisitResponse {
      smooth_in: 0.5,
      hold: 1.0,
      smooth_out: Some(3.0),
      curve_in: Curve::EaseOut,
      curve_out: Curve::EaseIn,
      ..VisitResponse::new("path", 0.0, 1.0)
    };
    let counted = VisitResponse { overlap: Overlap::Add, ..VisitResponse::new("walk", 0.0, 20.0) };
    let stops = vec![Stop::new(0.0, Color(Rgba([255, 220, 0, 255]))), Stop::new(1.0, Color(Rgba([255, 255, 255, 255])))];
    let fill = Fill::new(FillNode::Gradient { stops, style: GradientStyle::Linear })
      .transform(Transform::Translate(Vec2::new(-0.5, 0.0).into()))
      .transform(Transform::Fit("path".into()))
      .adjust(ColorAdjust::Opacity(Prop::Visit(shown)))
      .adjust(ColorAdjust::HueShift(Prop::Visit(counted)));
    let layer = Layer::new(fill).graph(GraphMask::new(traversals));
    Scene::new(Surface::Area, vec![layer]).timing(Timing::new(4.0, Cycle::Forever))
  }

  /// Colors on a 3 by 3 graph of nodes "0" to "8", 20 apart, over a few moments
  fn sample_graph(scene: &Scene) -> Vec<Rgba<u8>> {
    let params = Params::from([("hit".into(), Value::Vec2(Vec2::new(45.0, 45.0))), ("speed".into(), Value::Float(3.0))]);
    let samples: Vec<Sample> = (0..9).map(|i| Sample::new(Vec2::new((i % 3) as f32 * 20.0, (i / 3) as f32 * 20.0), 1.0).at_node(i)).collect();
    let nodes = samples.iter().enumerate().map(|(i, sample)| GraphNode::new(i.to_string(), sample.pos)).collect();
    let graph = Graph::build(nodes);
    [0.0, 0.3, 0.8, 1.5, 2.6]
      .into_iter()
      .flat_map(|time| {
        let frame = FrameContext { time, playback_seed: 3, params: &params, images: Images::none(), size: Vec2::new(100.0, 50.0), surface: scene.surface, visits: None };
        let visits = scene.visits(&graph, &frame);
        let frame = FrameContext { visits: Some(&visits), ..frame };
        samples.iter().map(|sample| scene.sample(sample, &frame)).collect::<Vec<_>>()
      })
      .collect()
  }

  #[test]
  fn a_graph_round_trip_samples_the_same() {
    let scene = on_a_graph();
    let written = save(&scene).unwrap();
    let read = load(&written).unwrap();
    let colors = sample_graph(&read);
    assert_eq!(colors, sample_graph(&scene), "written as:\n{written}");
    assert!(colors.iter().any(|color| color[3] > 0), "nothing was shown");
    assert_eq!(save(&read).unwrap(), written);
  }

  #[test]
  fn reads_a_hand_written_graph() {
    let scene = load(
      r#"(timing: (duration: 2.0), layers: [(
        fill: (base: Solid(Fixed(255, 0, 0, 255)), adjust: [Opacity(Visit(traversal: "bolt", from: 0.0, to: 1.0, smooth_out: 3.0))]),
        graph: (edges: (rule: RelativeNeighborhood, max_length: 3.0), traversals: [(name: "bolt", from: Name("0"), algorithm: ShortestPath(to: Nearest(Fixed(40.0, 40.0))), speed: Rate(Fixed(30.0)))]),
      )])"#,
    )
    .unwrap();
    let mask = scene.layers[0].graph.as_ref().unwrap();
    assert!(matches!(&mask.traversals[0].algorithm, TraversalAlgorithm::ShortestPath { wander: Prop::Fixed(wander), .. } if *wander == 0.0));
    let ColorAdjust::Opacity(Prop::Visit(response)) = &scene.layers[0].fill.adjust[0] else {
      panic!("{scene:?}");
    };
    assert_eq!((response.smooth_in, response.smooth_out, response.overlap), (0.0, Some(3.0), Overlap::Max));
  }

  #[test]
  fn a_newer_file_is_refused() {
    assert!(matches!(check_version(VERSION + 1), Err(FormatError::Newer { .. })));
    assert!(check_version(VERSION).is_ok());
  }
}
