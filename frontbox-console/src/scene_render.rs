use frontbox_core::prelude::{LED, LedLookup, LedQ, ReferencePlane, Rgb, Vec2};
use frontbox_gfx::layout::{Grid, Line};
use frontbox_gfx::animation::Cycle;
use frontbox_gfx::prelude::{Rgba, RgbaColor};
use frontbox_gfx::scene::{Frame, Params, Sample, Scene, Timing, Viewport, file};
use frontbox_led::{DEFAULT_LED_RADIUS, place_on_plane};
use std::collections::BTreeMap;

use crate::protocol::{RenderedArea, RenderedClip, RenderedScene, SceneSize, SceneSource};

/// The most points one render will sample. A render costs the machine time and anything on the network can ask for
/// one. This is far more than any DMD or list of LEDs.
const MAX_SAMPLES: usize = 1 << 18;

/// The most points one clip will sample, across all of its frames. About 30 MB of colors.
const MAX_CLIP_SAMPLES: usize = 1 << 23;

const CLIP_FRAMES_PER_SECOND: f32 = 30.0;

/// How far before the end of a cycle in the middle of a scene to show its last moment, as a fraction of the cycle. Any
/// nearer and the time rounds into the next cycle.
const END_OF_CYCLE: f32 = 1e-4;

/// The LEDs to play a scene on, mirroring how `LedScene` is given them
pub(crate) struct LedTarget {
  pub leds: LedLookup,
  /// What an `Area` scene plays on. A `Sequence` scene doesn't use one.
  pub plane: Option<&'static ReferencePlane>,
  pub query: LedQ,
}

/// Renders a moment of a scene with its default parameters, to whichever of the sizes suits its viewport. The reply has
/// the scene in both forms, as this is the only place RON is read or written for a client. The error is for showing to
/// whoever wrote the scene.
pub(crate) fn render_scene(
  source: SceneSource,
  size: SceneSize,
  target: LedTarget,
  time: f32,
) -> Result<RenderedScene, String> {
  let scene = read(source)?;
  let (width, height, device_size) = points(&scene, size, &target)?;
  let cycle = one_cycle(&scene);
  let params = Params::new();
  let frame = Frame { time: cycle_time(&cycle, time), take: 0, params: &params, size: device_size };

  let (colors, area, outlines) = match scene.viewport {
    Viewport::Sequence => (sample(&cycle, Line::new(width).samples(), &frame), None, Vec::new()),
    Viewport::Area => {
      let grid = Grid::new(width, height, device_size);
      let outlines = cycle.outlines(&frame);
      let outlines = outlines.iter().map(|outline| outline.iter().map(|p| p.to_array()).collect()).collect();
      (sample(&cycle, grid.samples(), &frame), Some(rendered_area(&grid)), outlines)
    }
  };
  let led_colors = led_colors(&cycle, &target, &frame);
  let ron = file::save(&scene).map_err(|err| err.to_string())?;

  Ok(RenderedScene { scene, ron, width, height, colors, area, outlines, led_colors })
}

/// Renders every frame of a cycle of a scene, for a client to play without coming back to the machine. See
/// [`RenderedClip`] for how it's laid out.
pub(crate) fn render_clip(
  id: u32,
  source: SceneSource,
  size: SceneSize,
  target: LedTarget,
) -> Result<Vec<u8>, String> {
  let scene = read(source)?;
  let Some(timing) = scene.timing else {
    return Err("a static scene has nothing to play".into());
  };
  let (width, height, device_size) = points(&scene, size, &target)?;
  let scene = one_cycle(&scene);

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
  let header = RenderedClip { id, frames, width, height, leds };
  let header = serde_json::to_vec(&header).map_err(|err| err.to_string())?;

  let mut clip = Vec::with_capacity(4 + header.len() + frames * (samples.len() * 4 + placed.len() * 3));
  clip.extend((header.len() as u32).to_le_bytes());
  clip.extend(header);

  let mut led_colors = Vec::with_capacity(frames * placed.len() * 3);
  let params = Params::new();
  for index in 0..frames {
    let time = cycle_time(&scene, timing.duration * index as f32 / (frames - 1) as f32);
    let frame = Frame { time, take: 0, params: &params, size: device_size };
    clip.extend(samples.iter().flat_map(|at| scene.sample(at, &frame).0));
    led_colors.extend(placed.iter().flat_map(|(_, at)| lit(scene.sample(at, &frame))));
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

/// The scene for showing one of its cycles, as a previewed scene is always somewhere in one however many times it
/// repeats. One that repeats gets a cycle either side, so that random picks smooth across the ends as they do in play.
fn one_cycle(scene: &Scene) -> Scene {
  let mut scene = scene.clone();
  if let Some(timing) = scene.timing.as_mut().filter(|timing| timing.cycle != Cycle::Once) {
    timing.cycle = Cycle::Times(3);
  }
  scene
}

/// The time in a scene from [`one_cycle`] of a time in the cycle it's showing, even the very end
fn cycle_time(scene: &Scene, time: f32) -> f32 {
  match scene.timing {
    Some(Timing { duration, cycle: Cycle::Times(_) }) => duration + time.min(duration * (1.0 - END_OF_CYCLE)),
    _ => time,
  }
}

fn rendered_area(grid: &Grid) -> RenderedArea {
  let covers = grid.covers();
  let size = covers.size();
  RenderedArea { x: covers.min.x, y: covers.min.y, width: size.x, height: size.y }
}

fn sample(scene: &Scene, samples: impl Iterator<Item = Sample>, frame: &Frame) -> Vec<u8> {
  samples.flat_map(|at| scene.sample(&at, frame).0).collect()
}

/// What each LED shows while the scene plays on it alone
fn led_colors(scene: &Scene, target: &LedTarget, frame: &Frame) -> BTreeMap<String, Rgb> {
  placed(scene.viewport, target)
    .iter()
    .map(|(led, at)| {
      let [r, g, b] = lit(scene.sample(at, frame));
      (led.name.clone(), Rgb::new(r, g, b))
    })
    .collect()
}

/// A color as an LED shows it, where nothing behind it is the LED being off
fn lit(color: Rgba<u8>) -> [u8; 3] {
  let [r, g, b, _] = color.composite_over(Rgba::black()).0;
  [r, g, b]
}

fn placed(viewport: Viewport, target: &LedTarget) -> Vec<(&LED, Sample)> {
  let LedTarget { leds, plane, query } = target;
  match (viewport, plane) {
    (Viewport::Sequence, _) => {
      let matching: Vec<_> = leds.query_iter(query).collect();
      let line = Line::new(matching.len());
      matching.into_iter().zip(line.samples()).collect()
    }
    (Viewport::Area, Some(plane)) => place_on_plane(leds, plane, query, DEFAULT_LED_RADIUS),
    (Viewport::Area, None) => Vec::new(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use frontbox_core::prelude::{ExpAddress, LedAddress, Vec3};
  use std::sync::LazyLock;

  const RED_TO_BLUE: &str = r#"(
    version: 1,
    viewport: Sequence,
    layers: [
      (paint: (base: Gradient(stops: [
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
    LedTarget { leds: LedLookup::default(), plane: None, query: LedQ::every() }
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
      (paint: (base: Gradient(stops: [
        (at: 0.0, color: Fixed(255, 0, 0, 255)),
        (at: 1.0, color: Fixed(0, 0, 255, 255)),
      ]))),
    ],
  )"#;

  fn colors(source: &str, target: LedTarget) -> BTreeMap<String, Rgb> {
    render_scene(ron(source), SIZE, target, 0.0).unwrap().led_colors
  }

  #[test]
  fn a_sequence_spreads_over_the_leds_in_order() {
    let target = LedTarget { leds: leds(&[None, None, None]), ..no_leds() };
    let colors = colors(RED_TO_BLUE, target);
    assert_eq!(colors["led0"], Rgb::new(255, 0, 0));
    assert_eq!(colors["led2"], Rgb::new(0, 0, 255));
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
  fn leds_off_the_plane_arent_colored() {
    let target = LedTarget {
      leds: leds(&[Some(Vec3::new(5.0, 5.0, 0.0)), None]),
      plane: Some(&PLANE),
      ..no_leds()
    };
    assert!(colors(AREA, target).is_empty());
  }

  #[test]
  fn an_area_without_a_plane_colors_nothing() {
    let target = LedTarget { leds: leds(&[Some(Vec3::new(105.0, 5.0, 0.0))]), ..no_leds() };
    assert!(colors(AREA, target).is_empty());
  }

  #[test]
  fn alpha_dims_an_led_toward_off() {
    let source = "(version: 1, viewport: Sequence, layers: [(paint: (base: Solid(Fixed(255, 0, 0, 128))))])";
    let target = LedTarget { leds: leds(&[None]), ..no_leds() };
    assert_eq!(colors(source, target)["led0"], Rgb::new(127, 0, 0));
  }

  #[test]
  fn a_sequence_renders_to_leds() {
    let rendered = render_scene(ron(RED_TO_BLUE), SIZE, no_leds(), 0.0).unwrap();
    assert_eq!((rendered.width, rendered.height), (3, 1));
    assert_eq!(rendered.colors.len(), 12);
    assert_eq!(rendered.colors[..4], [255, 0, 0, 255]);
    assert_eq!(rendered.colors[8..], [0, 0, 255, 255]);
  }

  #[test]
  fn an_area_renders_to_pixels() {
    let source = "(version: 1, viewport: Area, layers: [])";
    let rendered = render_scene(ron(source), SIZE, no_leds(), 0.0).unwrap();
    assert_eq!((rendered.width, rendered.height), (4, 2));
  }

  #[test]
  fn an_area_says_what_a_render_covers() {
    // 4 x 2 is twice as wide as this square plane, so half of it falls either side
    let target = LedTarget { plane: Some(&SQUARE), ..no_leds() };
    let source = "(version: 1, viewport: Area, layers: [])";
    let area = render_scene(ron(source), SIZE, target, 0.0).unwrap().area.unwrap();
    assert_eq!((area.x, area.y, area.width, area.height), (-5.0, 0.0, 20.0, 10.0));
  }

  #[test]
  fn an_area_with_no_plane_is_the_size_of_the_render() {
    let source = "(version: 1, viewport: Area, layers: [])";
    let area = render_scene(ron(source), SIZE, no_leds(), 0.0).unwrap().area.unwrap();
    assert_eq!((area.x, area.y, area.width, area.height), (0.0, 0.0, 4.0, 2.0));
  }

  #[test]
  fn a_layer_with_no_shape_outlines_the_plane() {
    let target = LedTarget { plane: Some(&SQUARE), ..no_leds() };
    let source = "(version: 1, viewport: Area, layers: [(paint: (base: Solid(Fixed(1, 2, 3, 255))))])";
    let outlines = render_scene(ron(source), SIZE, target, 0.0).unwrap().outlines;
    assert_eq!(outlines, [[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]]);
  }

  #[test]
  fn a_shape_outlines_where_it_is_on_the_plane() {
    let target = LedTarget { plane: Some(&SQUARE), ..no_leds() };
    let source = r#"(
      version: 1,
      viewport: Area,
      layers: [(
        shape: (base: Polygon(points: [Fixed(0.0, 0.0), Fixed(1.0, 0.0), Fixed(0.0, 1.0)]), warp: [Translate(Fixed(2.0, 3.0))]),
        paint: (base: Solid(Fixed(1, 2, 3, 255))),
      )],
    )"#;
    let outlines = render_scene(ron(source), SIZE, target, 0.0).unwrap().outlines;
    assert_eq!(outlines, [[[2.0, 3.0], [3.0, 3.0], [2.0, 4.0]]]);
  }

  #[test]
  fn a_sequence_has_no_outlines() {
    assert!(render_scene(ron(RED_TO_BLUE), SIZE, no_leds(), 0.0).unwrap().outlines.is_empty());
  }

  #[test]
  fn a_sequence_has_no_area() {
    assert!(render_scene(ron(RED_TO_BLUE), SIZE, no_leds(), 0.0).unwrap().area.is_none());
  }

  #[test]
  fn grid_fits_an_area() {
    // a circle in the center of a 100 x 50 viewport, at far less resolution than it has units
    let source = r#"(
      version: 1,
      viewport: Area,
      layers: [(
        shape: (base: Ellipse, warp: [Scale(Fixed(40.0, 40.0)), Translate(Fixed(50.0, 25.0))]),
        paint: (base: Solid(Fixed(255, 0, 0, 255))),
      )],
    )"#;
    let target = LedTarget { plane: Some(&WIDE), ..no_leds() };
    let rendered = render_scene(ron(source), SceneSize { leds: 0, width: 3, height: 3 }, target, 0.0).unwrap();
    let pixel = |x: usize, y: usize| &rendered.colors[(y * 3 + x) * 4..][..4];
    assert_eq!(pixel(1, 1), [255, 0, 0, 255]);
    assert_eq!(pixel(0, 0), [0, 0, 0, 0]);
  }

  #[test]
  fn reports_where_a_scene_is_wrong() {
    let error = render_scene(ron("(version: 1, viewport: Sequence, layers: [(paint: (base: Nope))])"), SIZE, no_leds(), 0.0);
    assert!(error.err().unwrap().contains("1:"));
  }

  #[test]
  fn refuses_a_size_too_large_to_sample() {
    let size = SceneSize { leds: usize::MAX, ..SIZE };
    assert!(render_scene(ron(RED_TO_BLUE), size, no_leds(), 0.0).is_err());
  }

  #[test]
  fn a_definition_renders_like_its_ron() {
    let from_ron = render_scene(ron(RED_TO_BLUE), SIZE, no_leds(), 0.0).unwrap();
    let from_definition = render_scene(SceneSource::Definition(from_ron.scene), SIZE, no_leds(), 0.0).unwrap();
    assert_eq!(from_definition.colors, from_ron.colors);
    assert_eq!(from_definition.ron, from_ron.ron);
  }

  #[test]
  fn a_definition_is_held_to_the_same_rules_as_a_file() {
    let source = r#"(
      version: 2,
      viewport: Area,
      timing: (duration: 1.0),
      layers: [(paint: (base: Solid(Track(keys: [(at: 0.0, value: (0, 0, 0, 255))]))))],
    )"#;
    let mut scene = render_scene(ron(source), SIZE, no_leds(), 0.0).unwrap().scene;
    scene.timing = None;
    let error = render_scene(SceneSource::Definition(scene), SIZE, no_leds(), 0.0).err().unwrap();
    assert!(error.contains("timing"), "{error}");
  }

  /// Goes from black to red over two seconds
  fn rising(cycle: &str) -> SceneSource {
    ron(&format!(
      r#"(
        version: 2,
        viewport: Sequence,
        timing: (duration: 2.0, cycle: {cycle}),
        layers: [(paint: (base: Solid(Track(keys: [
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
    let rendered = render_scene(rising("Once"), SIZE, no_leds(), 1.0).unwrap();
    assert_eq!(rendered.colors[..4], [100, 0, 0, 255]);
  }

  #[test]
  fn the_end_of_a_cycle_is_its_last_moment_however_often_it_repeats() {
    let rendered = render_scene(rising("Forever"), SIZE, no_leds(), 2.0).unwrap();
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
          layers: [(paint: (base: Solid(Random(
            picks: [(at: 0.0, smooth_in: 1.0)],
            spread: Color(hue: (0.0, 360.0), brightness: (1.0, 1.0)),
          ))))],
        )"#
      ))
    };
    let colors = |source, time| render_scene(source, SIZE, no_leds(), time).unwrap().colors;
    assert_ne!(colors(scene("Forever"), 0.25), colors(scene("Forever"), 0.75));
    assert_eq!(colors(scene("Once"), 0.25), colors(scene("Once"), 0.75));
  }

  #[test]
  fn a_clip_runs_from_the_start_of_a_cycle_to_its_end() {
    let clip = render_clip(7, rising("Forever"), SIZE, no_leds()).unwrap();
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
      layers: [(paint: (base: Solid(Fixed(200, 0, 0, 128))))],
    )"#;
    let target = LedTarget { leds: leds(&[None, None]), ..no_leds() };
    let clip = render_clip(0, ron(source), SIZE, target).unwrap();
    let (header, colors) = read_clip(&clip);
    assert_eq!(header["leds"], serde_json::json!(["led0", "led1"]));

    let frames = header["frames"].as_u64().unwrap() as usize;
    let leds = &colors[frames * 3 * 4..];
    assert_eq!(leds.len(), frames * 2 * 3);
    assert_eq!(leds[..6], [100, 0, 0, 100, 0, 0]);
  }

  #[test]
  fn a_static_scene_has_no_clip() {
    assert!(render_clip(0, ron(RED_TO_BLUE), SIZE, no_leds()).is_err());
  }

  #[test]
  fn refuses_a_clip_too_big_to_send() {
    let size = SceneSize { leds: 1 << 18, width: 0, height: 0 };
    let error = render_clip(0, rising("Once"), size, no_leds()).unwrap_err();
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
          warp: [Rotate(Track(keys: [(at: 0.0, value: 0.0), (at: 1.0, value: 90.0, curve: Steps(4))], cycle: Times(2)))],
        ),
        paint: (base: Solid(Param("tint")), adjust: [HueShift(Param("size"))]),
      )],
    )"#;
    let rendered = render_scene(ron(source), SIZE, no_leds(), 0.0).unwrap();
    let json = serde_json::to_string(&rendered.scene).unwrap();
    let back = render_scene(SceneSource::Definition(serde_json::from_str(&json).unwrap()), SIZE, no_leds(), 0.0).unwrap();
    assert_eq!(back.ron, rendered.ron);
  }
}
