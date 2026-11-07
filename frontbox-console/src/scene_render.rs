use frontbox_core::prelude::{LED, LedLookup, LedQ, ReferencePlane, Rgb, Vec2};
use frontbox_gfx::layout::{Grid, Line};
use frontbox_gfx::scene::{Bounds, Content, Frame, Images, Params, Sample, Scene, ShapePose, Viewport, file};
use frontbox_led::{led_color, place_leds};
use std::collections::BTreeMap;

use crate::protocol::{RenderedArea, RenderedClip, RenderedScene, RenderedShape, SceneSize, SceneSource};

/// The most points one render will sample. A render costs the machine time and anything on the network can ask for
/// one. This is far more than any DMD or list of LEDs.
const MAX_SAMPLES: usize = 1 << 18;

/// The most points one clip will sample, across all of its frames. About 30 MB of colors.
const MAX_CLIP_SAMPLES: usize = 1 << 23;

const CLIP_FRAMES_PER_SECOND: f32 = 30.0;

/// The LEDs to play a scene on, mirroring how `LedScene` is given them
pub(crate) struct LedTarget {
  pub leds: LedLookup,
  /// What an `Area` scene plays on. A `Sequence` scene doesn't use one.
  pub plane: Option<&'static ReferencePlane>,
  pub query: LedQ,
  pub radius: f32,
}

/// Renders a moment of a scene with its default parameters, to whichever of the sizes suits its viewport. Without a
/// cycle, that moment is in one from the middle of the scene. The reply has the scene in both forms, as this is the only
/// place RON is read or written for a client. The error is for showing to whoever wrote the scene.
pub(crate) fn render_scene(
  source: SceneSource,
  size: SceneSize,
  target: LedTarget,
  content: Option<Content>,
  time: f32,
  cycle: Option<u32>,
) -> Result<RenderedScene, String> {
  let scene = read(source)?;
  let (width, height, device_size) = points(&scene, size, &target)?;
  let images = images(&scene, content)?;
  let (shown, time) = match cycle {
    Some(cycle) => (scene.clone(), scene.cycle_time(cycle, time)),
    None => {
      let middle = scene.middle_cycle();
      let time = middle.middle_cycle_time(time);
      (middle, time)
    }
  };
  let params = Params::new();
  let frame = Frame { time, take: 0, params: &params, images: &images, size: device_size };

  let (colors, covers) = match scene.viewport {
    Viewport::Sequence => {
      let line = Line::new(width);
      (sample(&shown, line.samples(), &frame), line.covers())
    }
    Viewport::Area => {
      let grid = Grid::new(width, height, device_size);
      (sample(&shown, grid.samples(), &frame), grid.covers())
    }
  };
  let area = rendered_area(covers);
  let outlines = shown.outlines(&frame).iter().map(|outline| outline.iter().map(|p| p.to_array()).collect()).collect();
  let shapes = shown.shape_poses(&frame).iter().map(|pose| pose.as_ref().map(rendered_shape)).collect();
  let led_colors = led_colors(&shown, &target, &frame);
  let ron = file::save(&scene).map_err(|err| err.to_string())?;

  Ok(RenderedScene { scene, ron, width, height, colors, area, outlines, shapes, led_colors })
}

/// Renders every frame of one cycle of a scene, for a client to play without coming back to the machine until the next.
/// See [`RenderedClip`] for how it's laid out.
pub(crate) fn render_clip(
  id: u32,
  source: SceneSource,
  size: SceneSize,
  target: LedTarget,
  content: Option<Content>,
  cycle: u32,
) -> Result<Vec<u8>, String> {
  let scene = read(source)?;
  let Some(timing) = scene.timing else {
    return Err("a static scene has nothing to play".into());
  };
  let (width, height, device_size) = points(&scene, size, &target)?;

  let samples: Vec<Sample> = match scene.viewport {
    Viewport::Sequence => Line::new(width).samples().collect(),
    Viewport::Area => Grid::new(width, height, device_size).samples().collect(),
  };
  let placed = placed(scene.viewport, &target);
  // a frame at each end, so that a clip finishes on the scene's last moment
  let frames = ((timing.duration * CLIP_FRAMES_PER_SECOND).ceil() as usize).saturating_add(1);
  let total = (samples.len() + placed.len()).saturating_mul(frames);
  if total > MAX_CLIP_SAMPLES {
    return Err(format!("this clip is {total} points, more than the {MAX_CLIP_SAMPLES} one can sample"));
  }

  let leds = placed.iter().map(|(led, _)| led.name.clone()).collect();
  let images = images(&scene, content)?;
  let header = RenderedClip { id, frames, width, height, leds };
  let header = serde_json::to_vec(&header).map_err(|err| err.to_string())?;

  let mut clip = Vec::with_capacity(4 + header.len() + frames * (samples.len() * 4 + placed.len() * 3));
  clip.extend((header.len() as u32).to_le_bytes());
  clip.extend(header);

  let mut led_colors = Vec::with_capacity(frames * placed.len() * 3);
  let params = Params::new();
  for index in 0..frames {
    let time = scene.cycle_time(cycle, timing.duration * index as f32 / (frames - 1) as f32);
    let frame = Frame { time, take: 0, params: &params, images: &images, size: device_size };
    clip.extend(samples.iter().flat_map(|at| scene.sample(at, &frame).0));
    led_colors.extend(placed.iter().flat_map(|(_, at)| {
      let Rgb { r, g, b } = led_color(scene.sample(at, &frame));
      [r, g, b]
    }));
  }
  clip.extend(led_colors);
  Ok(clip)
}

fn read(source: SceneSource) -> Result<Scene, String> {
  let scene = match source {
    SceneSource::Ron(ron) => file::load(&ron).map_err(|err| err.to_string())?,
    SceneSource::Definition(scene) => {
      scene.check().map_err(|err| err.to_string())?;
      scene
    }
  };
  Ok(scene)
}

/// The images a scene draws. Each is only decoded the first time, so this is cheap for every frame the client asks for.
fn images(scene: &Scene, content: Option<Content>) -> Result<Images, String> {
  match content {
    Some(content) => content.images_for(scene).map_err(|err| err.to_string()),
    None if scene.image_names().next().is_some() => Err("this machine has no content folder".into()),
    None => Ok(Images::new()),
  }
}

/// How many points to sample across and down, and the size the device playing the scene would give it
fn points(scene: &Scene, size: SceneSize, target: &LedTarget) -> Result<(usize, usize, Vec2), String> {
  let (width, height) = match scene.viewport {
    Viewport::Sequence => (size.leds, 1),
    Viewport::Area => (size.width, size.height),
  };
  if width.saturating_mul(height) > MAX_SAMPLES {
    return Err(format!(
      "{width} x {height} is more than the {MAX_SAMPLES} points a render can sample"
    ));
  }
  let device_size = target.plane.map_or(Vec2::new(width as f32, height as f32), |plane| plane.extent);
  Ok((width, height, device_size))
}

fn rendered_area(covers: Bounds) -> RenderedArea {
  let size = covers.size();
  RenderedArea { x: covers.min.x, y: covers.min.y, width: size.x, height: size.y }
}

fn rendered_shape(pose: &ShapePose) -> RenderedShape {
  RenderedShape {
    transforms: pose.transforms.iter().map(|transform| transform.to_cols_array()).collect(),
    min: pose.bounds.min.to_array(),
    max: pose.bounds.max.to_array(),
    points: pose.points.iter().map(|p| p.to_array()).collect(),
    outline: pose.outline.iter().map(|p| p.to_array()).collect(),
    paths: pose.paths.iter().map(|path| path.as_ref().map(rendered_shape)).collect(),
  }
}

fn sample(scene: &Scene, samples: impl Iterator<Item = Sample>, frame: &Frame) -> Vec<u8> {
  samples.flat_map(|at| scene.sample(&at, frame).0).collect()
}

/// What each LED shows while the scene plays on it alone
fn led_colors(scene: &Scene, target: &LedTarget, frame: &Frame) -> BTreeMap<String, Rgb> {
  placed(scene.viewport, target)
    .iter()
    .map(|(led, at)| (led.name.clone(), led_color(scene.sample(at, frame))))
    .collect()
}

fn placed(viewport: Viewport, target: &LedTarget) -> Vec<(&LED, Sample)> {
  let LedTarget { leds, plane, query, radius } = target;
  place_leds(viewport, leds, *plane, query, *radius)
}

#[cfg(test)]
mod tests {
  use super::*;
  use frontbox_core::prelude::{DEFAULT_LED_RADIUS, ExpAddress, LedAddress, Vec3};
  use std::sync::LazyLock;

  const RED_TO_BLUE: &str = r#"(
    version: 1,
    viewport: Sequence,
    layers: [
      (fill: (base: Gradient(stops: [
        (at: 0.0, color: Fixed(255, 0, 0, 255)),
        (at: 1.0, color: Fixed(0, 0, 255, 255)),
      ]))),
    ],
  )"#;

  const SIZE: SceneSize = SceneSize { leds: 3, width: 4, height: 2 };

  fn ron(source: &str) -> SceneSource {
    SceneSource::Ron(source.into())
  }

  fn no_leds() -> LedTarget {
    LedTarget { leds: LedLookup::default(), plane: None, query: LedQ::every(), radius: DEFAULT_LED_RADIUS }
  }

  /// LEDs in hardware order, with these locations
  fn leds(locations: &[Option<Vec3>]) -> LedLookup {
    let mut leds = LedLookup::default();
    for (i, &location) in locations.iter().enumerate() {
      let name = format!("led{i}");
      let address = LedAddress::new(ExpAddress::default(), i as u8);
      leds.insert(name.clone(), LED { name, address, location, ..Default::default() });
    }
    leds
  }

  static PLANE: LazyLock<ReferencePlane> = LazyLock::new(|| {
    ReferencePlane::new("Plane").origin(Vec3::new(100.0, 0.0, 0.0)).extent(Vec2::new(10.0, 10.0)).build()
  });

  static SQUARE: LazyLock<ReferencePlane> =
    LazyLock::new(|| ReferencePlane::new("Square").extent(Vec2::new(10.0, 10.0)).build());

  static WIDE: LazyLock<ReferencePlane> =
    LazyLock::new(|| ReferencePlane::new("Wide").extent(Vec2::new(100.0, 50.0)).build());

  /// Red at x = 0, blue at x = 10
  const AREA: &str = r#"(
    version: 1,
    viewport: Area,
    layers: [
      (fill: (base: Gradient(stops: [
        (at: 0.0, color: Fixed(255, 0, 0, 255)),
        (at: 1.0, color: Fixed(0, 0, 255, 255)),
      ]))),
    ],
  )"#;

  fn colors(source: &str, target: LedTarget) -> BTreeMap<String, Rgb> {
    render_scene(ron(source), SIZE, target, None, 0.0, None).unwrap().led_colors
  }

  #[test]
  fn a_sequence_spreads_over_the_leds_in_order() {
    let target = LedTarget { leds: leds(&[None, None, None]), ..no_leds() };
    let colors = colors(RED_TO_BLUE, target);
    assert_eq!(colors["led0"], Rgb::new(255, 0, 0));
    assert_eq!(colors["led2"], Rgb::new(0, 0, 255));
  }

  #[test]
  fn a_sequence_spreads_over_only_the_chosen_leds_in_their_order() {
    let target = LedTarget { leds: leds(&[None, None, None]), query: LedQ::names(["led2", "led0"]), ..no_leds() };
    let colors = colors(RED_TO_BLUE, target);
    assert_eq!(colors.len(), 2);
    assert_eq!(colors["led2"], Rgb::new(255, 0, 0));
    assert_eq!(colors["led0"], Rgb::new(0, 0, 255));
  }

  #[test]
  fn an_area_colors_leds_by_where_they_are_on_the_plane() {
    let at = |x: f32| Some(Vec3::new(100.0 + x, 5.0, 0.0));
    let target = LedTarget { leds: leds(&[at(10.0), at(0.0)]), plane: Some(&PLANE), ..no_leds() };
    let colors = colors(AREA, target);
    assert_eq!(colors["led0"], Rgb::new(0, 0, 255));
    assert_eq!(colors["led1"], Rgb::new(255, 0, 0));
  }

  #[test]
  fn shape_edges_on_leds_fade_across_the_radius_the_machine_was_booted_with() {
    let source = r#"(
      version: 1,
      viewport: Area,
      layers: [(
        shape: (base: Ellipse(r1: Fixed(0.5), r2: Fixed(0.5)), transform: [Scale(Fixed(4.0, 4.0)), Translate(Fixed(5.0, 5.0))]),
        fill: (base: Solid(Fixed(255, 0, 0, 255))),
      )],
    )"#;
    let brightness = |radius| {
      let target = LedTarget {
        leds: leds(&[Some(Vec3::new(5.0, 6.95, 0.0))]),
        plane: Some(&SQUARE),
        radius,
        ..no_leds()
      };
      colors(source, target)["led0"].r
    };
    assert!(brightness(2.0) < brightness(DEFAULT_LED_RADIUS));
  }

  #[test]
  fn a_sequence_renders_to_leds() {
    let rendered = render_scene(ron(RED_TO_BLUE), SIZE, no_leds(), None, 0.0, None).unwrap();
    assert_eq!((rendered.width, rendered.height), (3, 1));
    assert_eq!(rendered.colors.len(), 12);
    assert_eq!(rendered.colors[..4], [255, 0, 0, 255]);
    assert_eq!(rendered.colors[8..], [0, 0, 255, 255]);
  }

  #[test]
  fn an_area_renders_to_pixels() {
    let source = "(version: 1, viewport: Area, layers: [])";
    let rendered = render_scene(ron(source), SIZE, no_leds(), None, 0.0, None).unwrap();
    assert_eq!((rendered.width, rendered.height), (4, 2));
  }

  #[test]
  fn an_area_says_what_a_render_covers() {
    // 4 x 2 is twice as wide as this square plane, so half of it falls either side
    let target = LedTarget { plane: Some(&SQUARE), ..no_leds() };
    let source = "(version: 1, viewport: Area, layers: [])";
    let area = render_scene(ron(source), SIZE, target, None, 0.0, None).unwrap().area;
    assert_eq!((area.x, area.y, area.width, area.height), (-5.0, 0.0, 20.0, 10.0));
  }

  #[test]
  fn an_area_with_no_plane_is_the_size_of_the_render() {
    let source = "(version: 1, viewport: Area, layers: [])";
    let area = render_scene(ron(source), SIZE, no_leds(), None, 0.0, None).unwrap().area;
    assert_eq!((area.x, area.y, area.width, area.height), (0.0, 0.0, 4.0, 2.0));
  }

  #[test]
  fn a_layer_with_no_shape_outlines_the_plane() {
    let target = LedTarget { plane: Some(&SQUARE), ..no_leds() };
    let source = "(version: 1, viewport: Area, layers: [(fill: (base: Solid(Fixed(1, 2, 3, 255))))])";
    let outlines = render_scene(ron(source), SIZE, target, None, 0.0, None).unwrap().outlines;
    assert_eq!(outlines, [[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]]);
  }

  #[test]
  fn a_shape_outlines_where_it_is_on_the_plane() {
    let target = LedTarget { plane: Some(&SQUARE), ..no_leds() };
    let source = r#"(
      version: 1,
      viewport: Area,
      layers: [(
        shape: (base: Polygon(points: [Fixed(0.0, 0.0), Fixed(1.0, 0.0), Fixed(0.0, 1.0)]), transform: [Translate(Fixed(2.0, 3.0))]),
        fill: (base: Solid(Fixed(1, 2, 3, 255))),
      )],
    )"#;
    let outlines = render_scene(ron(source), SIZE, target, None, 0.0, None).unwrap().outlines;
    assert_eq!(outlines, [[[2.0, 3.0], [3.0, 3.0], [2.0, 4.0]]]);
  }

  #[test]
  fn a_layer_with_no_shape_outlines_a_whole_sequence() {
    let outlines = render_scene(ron(RED_TO_BLUE), SIZE, no_leds(), None, 0.0, None).unwrap().outlines;
    assert_eq!(outlines, [[[0.0, -0.5], [1.0, -0.5], [1.0, 0.5], [0.0, 0.5]]]);
  }

  #[test]
  fn a_rendered_shape_has_its_transforms_at_that_moment() {
    let source = r#"(
      version: 1,
      viewport: Sequence,
      timing: (duration: 2.0),
      layers: [
        (fill: (base: Solid(Fixed(255, 0, 0, 255)))),
        (
          fill: (base: Solid(Fixed(255, 0, 0, 255))),
          shape: (
            base: Rect(width: Fixed(0.5), height: Fixed(0.25)),
            transform: [Translate(Track(keys: [(at: 0.0, value: (0.0, 0.0)), (at: 2.0, value: (1.0, 0.0))], cycle: Once))],
          ),
        ),
      ],
    )"#;
    let shapes = render_scene(ron(source), SIZE, no_leds(), None, 1.0, Some(0)).unwrap().shapes;
    assert!(shapes[0].is_none());
    let rect = shapes[1].as_ref().unwrap();
    assert_eq!(rect.transforms, [[1.0, 0.0, 0.0, 1.0, 0.5, 0.0]]);
    assert_eq!((rect.min, rect.max), ([-0.25, -0.125], [0.25, 0.125]));
  }

  #[test]
  fn a_sequences_area_gives_each_point_the_same_room() {
    let area = render_scene(ron(RED_TO_BLUE), SIZE, no_leds(), None, 0.0, None).unwrap().area;
    assert_eq!((area.x, area.y, area.width, area.height), (-0.25, -0.5, 1.5, 1.0));
  }

  #[test]
  fn grid_fits_an_area() {
    // a circle in the center of a 100 x 50 viewport, at far less resolution than it has units
    let source = r#"(
      version: 1,
      viewport: Area,
      layers: [(
        shape: (base: Ellipse(r1: Fixed(0.5), r2: Fixed(0.5)), transform: [Scale(Fixed(40.0, 40.0)), Translate(Fixed(50.0, 25.0))]),
        fill: (base: Solid(Fixed(255, 0, 0, 255))),
      )],
    )"#;
    let target = LedTarget { plane: Some(&WIDE), ..no_leds() };
    let rendered = render_scene(ron(source), SceneSize { leds: 0, width: 3, height: 3 }, target, None, 0.0, None).unwrap();
    let pixel = |x: usize, y: usize| &rendered.colors[(y * 3 + x) * 4..][..4];
    assert_eq!(pixel(1, 1), [255, 0, 0, 255]);
    assert_eq!(pixel(0, 0), [0, 0, 0, 0]);
  }

  #[test]
  fn reports_where_a_scene_is_wrong() {
    let error = render_scene(ron("(version: 1, viewport: Sequence, layers: [(fill: (base: Nope))])"), SIZE, no_leds(), None, 0.0, None);
    assert!(error.err().unwrap().contains("1:"));
  }

  #[test]
  fn refuses_a_size_too_large_to_sample() {
    let size = SceneSize { leds: usize::MAX, ..SIZE };
    assert!(render_scene(ron(RED_TO_BLUE), size, no_leds(), None, 0.0, None).is_err());
  }

  #[test]
  fn a_definition_renders_like_its_ron() {
    let from_ron = render_scene(ron(RED_TO_BLUE), SIZE, no_leds(), None, 0.0, None).unwrap();
    let from_definition = render_scene(SceneSource::Definition(from_ron.scene), SIZE, no_leds(), None, 0.0, None).unwrap();
    assert_eq!(from_definition.colors, from_ron.colors);
    assert_eq!(from_definition.ron, from_ron.ron);
  }

  #[test]
  fn a_definition_is_held_to_the_same_rules_as_a_file() {
    let source = r#"(
      version: 2,
      viewport: Area,
      timing: (duration: 1.0),
      layers: [(fill: (base: Solid(Track(keys: [(at: 0.0, value: (0, 0, 0, 255))]))))],
    )"#;
    let mut scene = render_scene(ron(source), SIZE, no_leds(), None, 0.0, None).unwrap().scene;
    scene.timing = None;
    let error = render_scene(SceneSource::Definition(scene), SIZE, no_leds(), None, 0.0, None).err().unwrap();
    assert!(error.contains("timing"), "{error}");
  }

  /// Goes from black to red over two seconds
  fn rising(cycle: &str) -> SceneSource {
    ron(&format!(
      r#"(
        version: 2,
        viewport: Sequence,
        timing: (duration: 2.0, cycle: {cycle}),
        layers: [(fill: (base: Solid(Track(keys: [
          (at: 0.0, value: (0, 0, 0, 255)),
          (at: 2.0, value: (200, 0, 0, 255)),
        ]))))],
      )"#
    ))
  }

  /// A clip's header, and its frames' colors
  fn read_clip(clip: &[u8]) -> (serde_json::Value, &[u8]) {
    let length = u32::from_le_bytes(clip[..4].try_into().unwrap()) as usize;
    (serde_json::from_slice(&clip[4..4 + length]).unwrap(), &clip[4 + length..])
  }

  #[test]
  fn renders_any_moment() {
    let rendered = render_scene(rising("Once"), SIZE, no_leds(), None, 1.0, None).unwrap();
    assert_eq!(rendered.colors[..4], [100, 0, 0, 255]);
  }

  #[test]
  fn the_end_of_a_cycle_is_its_last_moment_however_often_it_repeats() {
    let rendered = render_scene(rising("Forever"), SIZE, no_leds(), None, 2.0, None).unwrap();
    assert_eq!(rendered.colors[..4], [200, 0, 0, 255]);
    assert!(rendered.ron.contains("Forever"), "{}", rendered.ron);
  }

  #[test]
  fn a_preview_of_a_scene_that_repeats_smooths_across_its_cycles() {
    let scene = |cycle: &str| {
      ron(&format!(
        r#"(
          version: 5,
          viewport: Sequence,
          timing: (duration: 1.0, cycle: {cycle}),
          layers: [(fill: (base: Solid(Random(
            picks: [(at: 0.0, smooth_in: 1.0)],
            spread: Color(hue: (0.0, 360.0), brightness: (1.0, 1.0)),
          ))))],
        )"#
      ))
    };
    let colors = |source, time| render_scene(source, SIZE, no_leds(), None, time, None).unwrap().colors;
    assert_ne!(colors(scene("Forever"), 0.25), colors(scene("Forever"), 0.75));
    assert_eq!(colors(scene("Once"), 0.25), colors(scene("Once"), 0.75));
    assert_eq!(colors(scene("Times(1)"), 0.25), colors(scene("Times(1)"), 0.75));
  }

  /// A new color at the start of each cycle of a second
  fn picking(cycle: &str, smooth_in: f32) -> SceneSource {
    ron(&format!(
      r#"(
        version: 5,
        viewport: Sequence,
        timing: (duration: 1.0, cycle: {cycle}),
        layers: [(fill: (base: Solid(Random(
          picks: [(at: 0.0, smooth_in: {smooth_in})],
          spread: Color(hue: (0.0, 360.0), brightness: (1.0, 1.0)),
        ))))],
      )"#
    ))
  }

  /// A folder with a 2 by 1 image, red on the left and blue on the right, called `art/a.png`
  fn content(folder: &str) -> Content {
    let root = std::env::temp_dir().join(format!("frontbox-console-{}-{folder}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("art")).unwrap();
    image::RgbaImage::from_fn(2, 1, |x, _| image::Rgba(if x == 0 { [255, 0, 0, 255] } else { [0, 0, 255, 255] }))
      .save(root.join("art/a.png"))
      .unwrap();
    Content::new(root)
  }

  const IMAGE: &str = r#"(version: 1, viewport: Area, layers: [(fill: (base: Image(name: "art/a.png")))])"#;
  const TWO_POINTS: SceneSize = SceneSize { leds: 0, width: 2, height: 1 };

  #[test]
  fn a_scene_draws_its_images_from_the_content_folder() {
    let content = content("draws");
    let rendered = render_scene(ron(IMAGE), TWO_POINTS, no_leds(), Some(content.clone()), 0.0, None).unwrap();
    assert_eq!(rendered.colors, [255, 0, 0, 255, 0, 0, 255, 255]);
    let _ = std::fs::remove_dir_all(content.root());
  }

  #[test]
  fn a_clip_draws_its_images_too() {
    let content = content("clip");
    let source = r#"(version: 1, viewport: Area, timing: (duration: 0.1, cycle: Once), layers: [(fill: (base: Image(name: "art/a.png")))])"#;
    let clip = render_clip(0, ron(source), TWO_POINTS, no_leds(), Some(content.clone()), 0).unwrap();
    let header = u32::from_le_bytes(clip[..4].try_into().unwrap()) as usize;
    assert_eq!(clip[4 + header..][..8], [255, 0, 0, 255, 0, 0, 255, 255]);
    let _ = std::fs::remove_dir_all(content.root());
  }

  #[test]
  fn a_clip_plays_through_the_frames_of_a_gif() {
    let content = content("gif");
    let mut gif = Vec::new();
    let frame = |color| {
      let delay = image::Delay::from_numer_denom_ms(100, 1);
      image::Frame::from_parts(image::RgbaImage::from_pixel(2, 1, image::Rgba(color)), 0, 0, delay)
    };
    image::codecs::gif::GifEncoder::new(&mut gif)
      .encode_frames([frame([255, 0, 0, 255]), frame([0, 0, 255, 255])])
      .unwrap();
    content.add_image("a.gif", &gif).unwrap();

    let source = r#"(version: 1, viewport: Area, timing: (duration: 1.0, cycle: Once), layers: [(fill: (base: Image(
      name: "a.gif",
      frame: Track(keys: [(at: 0.0, value: 0.0), (at: 1.0, value: 2.0)]),
    )))])"#;
    let clip = render_clip(0, ron(source), TWO_POINTS, no_leds(), Some(content.clone()), 0).unwrap();
    let header = u32::from_le_bytes(clip[..4].try_into().unwrap()) as usize;
    let frames = &clip[4 + header..];
    assert_eq!(frames[..4], [255, 0, 0, 255]);
    assert_eq!(frames[frames.len() - 8..frames.len() - 4], [0, 0, 255, 255]);
    let _ = std::fs::remove_dir_all(content.root());
  }

  #[test]
  fn a_missing_image_is_an_error_naming_it() {
    let content = content("missing");
    let source = IMAGE.replace("art/a.png", "art/b.png");
    let error = render_scene(ron(&source), TWO_POINTS, no_leds(), Some(content.clone()), 0.0, None).err().unwrap();
    assert!(error.contains("art/b.png"), "{error}");
    let _ = std::fs::remove_dir_all(content.root());
  }

  #[test]
  fn a_machine_with_no_content_folder_says_so_only_when_a_scene_needs_one() {
    let error = render_scene(ron(IMAGE), TWO_POINTS, no_leds(), None, 0.0, None).err().unwrap();
    assert!(error.contains("no content folder"), "{error}");
    assert!(render_scene(ron(RED_TO_BLUE), SIZE, no_leds(), None, 0.0, None).is_ok());
  }

  #[test]
  fn each_cycle_of_a_scene_has_a_clip_of_its_own() {
    let clip = |cycle| render_clip(0, picking("Forever", 0.0), SIZE, no_leds(), None, cycle).unwrap();
    let first = clip(0);
    assert_eq!(first, clip(0));
    assert_ne!(read_clip(&first).1, read_clip(&clip(1)).1);
    assert_ne!(read_clip(&clip(1)).1, read_clip(&clip(2)).1);
  }

  #[test]
  fn the_first_cycle_of_a_clip_has_nothing_to_smooth_from() {
    let clip = render_clip(0, picking("Forever", 1.0), SIZE, no_leds(), None, 0).unwrap();
    let (header, colors) = read_clip(&clip);
    let frames = header["frames"].as_u64().unwrap() as usize;
    assert_eq!(colors[..12], colors[(frames - 1) * 12..]);
  }

  #[test]
  fn a_moment_of_a_cycle_is_what_its_clip_shows() {
    let clip = render_clip(0, picking("Forever", 1.0), SIZE, no_leds(), None, 4).unwrap();
    let (header, colors) = read_clip(&clip);
    let frames = header["frames"].as_u64().unwrap() as usize;
    let middle = (frames - 1) / 2;
    let moment = render_scene(picking("Forever", 1.0), SIZE, no_leds(), None, 0.5, Some(4)).unwrap();
    assert_eq!(moment.colors, colors[middle * 12..][..12]);
  }

  #[test]
  fn a_clip_runs_from_the_start_of_a_cycle_to_its_end() {
    let clip = render_clip(7, rising("Forever"), SIZE, no_leds(), None, 0).unwrap();
    let (header, colors) = read_clip(&clip);
    assert_eq!(header["id"], 7);
    assert_eq!(header["width"], 3);
    assert_eq!(header["height"], 1);

    let frames = header["frames"].as_u64().unwrap() as usize;
    assert_eq!(frames, 61);
    assert_eq!(colors.len(), frames * 3 * 4);
    assert_eq!(colors[..4], [0, 0, 0, 255]);
    assert_eq!(colors[30 * 3 * 4..][..4], [100, 0, 0, 255]);
    assert_eq!(colors[60 * 3 * 4..][..4], [200, 0, 0, 255]);
  }

  #[test]
  fn a_clip_has_the_leds_after_the_points_with_nothing_behind_them_as_off() {
    let source = r#"(
      version: 2,
      viewport: Sequence,
      timing: (duration: 0.1),
      layers: [(fill: (base: Solid(Fixed(200, 0, 0, 128))))],
    )"#;
    let target = LedTarget { leds: leds(&[None, None]), ..no_leds() };
    let clip = render_clip(0, ron(source), SIZE, target, None, 0).unwrap();
    let (header, colors) = read_clip(&clip);
    assert_eq!(header["leds"], serde_json::json!(["led0", "led1"]));

    let frames = header["frames"].as_u64().unwrap() as usize;
    let leds = &colors[frames * 3 * 4..];
    assert_eq!(leds.len(), frames * 2 * 3);
    assert_eq!(leds[..6], [100, 0, 0, 100, 0, 0]);
  }

  #[test]
  fn a_static_scene_has_no_clip() {
    assert!(render_clip(0, ron(RED_TO_BLUE), SIZE, no_leds(), None, 0).is_err());
  }

  #[test]
  fn refuses_a_clip_too_big_to_send() {
    let size = SceneSize { leds: 1 << 18, width: 0, height: 0 };
    let error = render_clip(0, rising("Once"), size, no_leds(), None, 0).unwrap_err();
    assert!(error.contains("more than"), "{error}");
  }

  #[test]
  fn a_scene_survives_the_trip_to_a_client_and_back() {
    let source = r#"(
      version: 2,
      params: {"tint": Color(1, 2, 3, 4), "size": Float(2.5), "at": Vec2(0.5, 0.25)},
      viewport: Area,
      timing: (duration: 2.0, cycle: Forever),
      layers: [(
        shape: (
          base: Polygon(points: [Fixed(0.0, 0.0), Param("at"), Fixed(1.0, 0.0)]),
          transform: [Rotate(Track(keys: [(at: 0.0, value: 0.0), (at: 1.0, value: 90.0, curve: Steps(4))], cycle: Times(2)))],
        ),
        fill: (base: Solid(Param("tint")), adjust: [HueShift(Param("size"))]),
      )],
    )"#;
    let rendered = render_scene(ron(source), SIZE, no_leds(), None, 0.0, None).unwrap();
    let json = serde_json::to_string(&rendered.scene).unwrap();
    let back = render_scene(SceneSource::Definition(serde_json::from_str(&json).unwrap()), SIZE, no_leds(), None, 0.0, None).unwrap();
    assert_eq!(back.ron, rendered.ron);
  }

  #[test]
  fn a_sweep_survives_the_editor_and_only_on_a_position() {
    let source = r#"(
      version: 7,
      viewport: Area,
      timing: (duration: 2.0, cycle: Forever),
      layers: [(
        shape: (
          base: Ellipse(r1: Fixed(1.0), r2: Fixed(1.0)),
          transform: [Translate(Sweep(path: (base: Rect(width: Fixed(4.0), height: Fixed(2.0)), transform: [Translate(Fixed(5.0, 5.0))]), start: 0.5, end: 1.5))],
        ),
        fill: (base: Solid(Fixed(255, 0, 0, 255))),
      )],
    )"#;
    let rendered = render_scene(ron(source), SIZE, no_leds(), None, 0.0, None).unwrap();
    let json = serde_json::to_string(&rendered.scene).unwrap();
    let back = render_scene(SceneSource::Definition(serde_json::from_str(&json).unwrap()), SIZE, no_leds(), None, 0.0, None).unwrap();
    assert_eq!(back.ron, rendered.ron);

    let path = rendered.shapes[0].as_ref().unwrap().paths[0].as_ref().unwrap();
    assert_eq!(path.outline[0], [3.0, 4.0]);

    let on_a_size = json.replace(r#""r1":{"Fixed":1.0}"#, &format!(r#""r1":{}"#, sweep_of(&json)));
    let error = serde_json::from_str::<Scene>(&on_a_size).unwrap_err().to_string();
    assert!(error.contains("only a position"), "{error}");
  }

  /// The first sweep in a scene as JSON
  fn sweep_of(json: &str) -> String {
    let scene: serde_json::Value = serde_json::from_str(json).unwrap();
    scene["layers"][0]["shape"]["transform"][0]["Translate"].to_string()
  }
}
