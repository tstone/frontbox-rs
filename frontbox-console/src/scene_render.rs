use frontbox_core::prelude::{LedLookup, LedQ, ReferencePlane, Rgb, Vec2};
use frontbox_gfx::prelude::{Rgba, RgbaColor};
use frontbox_gfx::layout::{Grid, Line};
use frontbox_gfx::scene::{Bounds, Content, Frame, GfxComposition, Images, Params, Sample, ShapePose, Viewport};
use frontbox_led::{LedScene, LedTargets, led_color, place_leds};
use std::collections::{BTreeMap, BTreeSet};

use crate::protocol::{RenderedArea, RenderedClip, RenderedScene, RenderedShape, SceneDefinition, SceneSize, SceneSource};

/// The most points one render will sample. A render costs the machine time and anything on the network can ask for
/// one. This is far more than any DMD or list of LEDs.
const MAX_SAMPLES: usize = 1 << 18;

/// The most points one clip will sample, across all of its frames. About 30 MB of colors.
const MAX_CLIP_SAMPLES: usize = 1 << 23;

const CLIP_FRAMES_PER_SECOND: f32 = 30.0;

/// The machine a scene is previewed on
pub(crate) struct LedSurface {
  pub leds: LedLookup,
  /// What an `Area` scene with no targets is previewed on. One with targets says its own.
  pub plane: Option<&'static ReferencePlane>,
  pub radius: f32,
}

/// Renders a moment of a scene with its default parameters, to whichever of the sizes suits its viewport. Without a
/// cycle, that moment is in one from the middle of the scene. The reply has the scene in both forms, as this is the only
/// place RON is read or written for a client. The error is for showing to whoever wrote the scene.
pub(crate) fn render_scene(
  source: SceneSource,
  size: SceneSize,
  surface: LedSurface,
  content: Option<Content>,
  time: f32,
  cycle: Option<u32>,
) -> Result<RenderedScene, String> {
  let (scene, preview) = read(source)?;
  let drawn = preview.as_ref().unwrap_or(&scene);
  let (width, height, device_size) = points(drawn, size, &surface)?;
  let composition = drawn.composition();
  let viewport = drawn.viewport();
  let images = images(composition, content)?;
  let (shown, time) = match cycle {
    Some(cycle) => (composition.clone(), composition.cycle_time(cycle, time)),
    None => {
      let middle = composition.middle_cycle();
      let time = middle.middle_cycle_time(time);
      (middle, time)
    }
  };
  let params = Params::new();
  let frame = Frame { time, take: 0, params: &params, images: &images, size: device_size, viewport, target: None };

  let (colors, covers) = match viewport {
    Viewport::Sequence => {
      let line = Line::new(width);
      let rows = strips(drawn);
      let colors = rows.iter().flat_map(|row| sample(&shown, line.samples(), &Frame { target: row.as_deref(), ..frame })).collect();
      (colors, line.covers())
    }
    Viewport::Area => {
      let grid = Grid::new(width, height, device_size);
      (sample(&shown, grid.samples(), &frame), grid.covers())
    }
  };
  let area = rendered_area(covers);
  let outlines = rendered_outlines(shown.outlines(&frame));
  let shapes = rendered_shapes(shown.shape_poses(&frame));
  let mask_outlines = rendered_outlines(shown.mask_outlines(&frame));
  let mask_shapes = rendered_shapes(shown.mask_poses(&frame));
  let led_colors = led_colors(&led_parts(drawn, &surface), &shown, &frame);
  let ron = scene.save().map_err(|err| err.to_string())?;

  Ok(RenderedScene { scene, ron, width, height, colors, area, outlines, shapes, mask_outlines, mask_shapes, led_colors })
}

/// Renders every frame of one cycle of a scene, for a client to play without coming back to the machine until the next.
/// See [`RenderedClip`] for how it's laid out.
pub(crate) fn render_clip(
  id: u32,
  source: SceneSource,
  size: SceneSize,
  surface: LedSurface,
  content: Option<Content>,
  cycle: u32,
) -> Result<Vec<u8>, String> {
  let (scene, preview) = read(source)?;
  let scene = preview.unwrap_or(scene);
  let composition = scene.composition();
  let viewport = scene.viewport();
  let Some(timing) = composition.timing else {
    return Err("a static scene has nothing to play".into());
  };
  let (width, height, device_size) = points(&scene, size, &surface)?;

  let samples: Vec<Sample> = match viewport {
    Viewport::Sequence => Line::new(width).samples().collect(),
    Viewport::Area => Grid::new(width, height, device_size).samples().collect(),
  };
  let rows = match viewport {
    Viewport::Sequence => strips(&scene),
    Viewport::Area => vec![None],
  };
  let parts = led_parts(&scene, &surface);
  let leds: BTreeSet<_> = parts.iter().flat_map(|part| part.placed.iter().map(|(name, _)| name.clone())).collect();
  // a frame at each end, so that a clip finishes on the scene's last moment
  let frames = ((timing.duration * CLIP_FRAMES_PER_SECOND).ceil() as usize).saturating_add(1);
  let points = samples.len().saturating_mul(rows.len());
  let total = points.saturating_add(leds.len()).saturating_mul(frames);
  if total > MAX_CLIP_SAMPLES {
    return Err(format!("this clip is {total} points, more than the {MAX_CLIP_SAMPLES} one can sample"));
  }

  let images = images(composition, content)?;
  let led_count = leds.len();
  let header = RenderedClip { id, frames, width, height, leds: leds.into_iter().collect() };
  let header = serde_json::to_vec(&header).map_err(|err| err.to_string())?;

  let mut clip = Vec::with_capacity(4 + header.len() + frames * (points * 4 + led_count * 3));
  clip.extend((header.len() as u32).to_le_bytes());
  clip.extend(header);

  let mut led_colors = Vec::with_capacity(frames * led_count * 3);
  let params = Params::new();
  for index in 0..frames {
    let time = composition.cycle_time(cycle, timing.duration * index as f32 / (frames - 1) as f32);
    let frame = Frame { time, take: 0, params: &params, images: &images, size: device_size, viewport, target: None };
    for row in &rows {
      let frame = Frame { target: row.as_deref(), ..frame };
      clip.extend(samples.iter().flat_map(|at| composition.sample(at, &frame).0));
    }
    // every LED is shown every frame, and in name order, as `leds` has them
    led_colors.extend(self::led_colors(&parts, composition, &frame).into_values().flat_map(|Rgb { r, g, b }| [r, g, b]));
  }
  clip.extend(led_colors);
  Ok(clip)
}

/// A scene as it's written to a file, which has to be one that can be played. RON is kept as it was typed, comments and
/// all.
pub(crate) fn scene_file(source: SceneSource) -> Result<String, String> {
  match source {
    SceneSource::Ron(ron) => {
      SceneDefinition::load(&ron).map_err(|err| err.to_string())?;
      Ok(ron)
    }
    SceneSource::Definition(scene) | SceneSource::Preview { scene, .. } => checked(scene)?.save().map_err(|err| err.to_string()),
  }
}

/// The scene, and what to draw in its place if it's only being previewed
fn read(source: SceneSource) -> Result<(SceneDefinition, Option<SceneDefinition>), String> {
  match source {
    SceneSource::Ron(ron) => Ok((SceneDefinition::load(&ron).map_err(|err| err.to_string())?, None)),
    SceneSource::Definition(scene) => Ok((checked(scene)?, None)),
    SceneSource::Preview { scene, shown } => Ok((checked(scene)?, Some(checked(shown)?))),
  }
}

fn checked(scene: SceneDefinition) -> Result<SceneDefinition, String> {
  scene.check()?;
  Ok(scene)
}

/// The images a scene draws. Each is only decoded the first time, so this is cheap for every frame the client asks for.
fn images(scene: &GfxComposition, content: Option<Content>) -> Result<Images, String> {
  match content {
    Some(content) => content.images_for(scene).map_err(|err| err.to_string()),
    None if scene.image_names().next().is_some() => Err("this machine has no content folder".into()),
    None => Ok(Images::new()),
  }
}

/// How many points to sample across and down, and the size the device playing the scene would give it
fn points(scene: &SceneDefinition, size: SceneSize, surface: &LedSurface) -> Result<(usize, usize, Vec2), String> {
  let (width, height) = match scene.viewport() {
    Viewport::Sequence => (size.leds, strips(scene).len()),
    Viewport::Area => (size.width, size.height),
  };
  if width.saturating_mul(height) > MAX_SAMPLES {
    return Err(format!(
      "{width} x {height} is more than the {MAX_SAMPLES} points a render can sample"
    ));
  }
  let device_size = shown_plane(scene, surface).map_or(Vec2::new(width as f32, height as f32), |plane| plane.extent);
  Ok((width, height, device_size))
}

/// The plane an `Area` scene is drawn on. With several, that's the first target's: the others are still on theirs, but
/// a render is of the one area.
fn shown_plane(scene: &SceneDefinition, surface: &LedSurface) -> Option<&'static ReferencePlane> {
  let SceneDefinition::Led(scene) = scene else {
    return None;
  };
  match scene.targets.iter().next() {
    Some((_, target)) => target.plane.as_deref().and_then(|plane| surface.leds.plane(plane)),
    None => surface.plane,
  }
}

fn rendered_area(covers: Bounds) -> RenderedArea {
  let size = covers.size();
  RenderedArea { x: covers.min.x, y: covers.min.y, width: size.x, height: size.y }
}

fn rendered_outlines(outlines: Vec<Vec<Vec2>>) -> Vec<Vec<[f32; 2]>> {
  outlines.iter().map(|outline| outline.iter().map(|p| p.to_array()).collect()).collect()
}

fn rendered_shapes(poses: Vec<Option<ShapePose>>) -> Vec<Option<RenderedShape>> {
  poses.iter().map(|pose| pose.as_ref().map(rendered_shape)).collect()
}

fn rendered_shape(pose: &ShapePose) -> RenderedShape {
  RenderedShape {
    transforms: pose.transforms.iter().map(|transform| transform.to_cols_array()).collect(),
    min: pose.bounds.min.to_array(),
    max: pose.bounds.max.to_array(),
    points: pose.points.iter().map(|p| p.to_array()).collect(),
    handles: pose.handles.iter().map(|p| p.to_array()).collect(),
    hole: pose.hole,
    outline: pose.outline.iter().map(|p| p.to_array()).collect(),
    paths: pose.paths.iter().map(|path| path.as_ref().map(rendered_shape)).collect(),
  }
}

fn sample(scene: &GfxComposition, samples: impl Iterator<Item = Sample>, frame: &Frame) -> Vec<u8> {
  samples.flat_map(|at| scene.sample(&at, frame).0).collect()
}

/// Which target each row of the strip shows, or with none named, the one row of the whole scene
fn strips(scene: &SceneDefinition) -> Vec<Option<String>> {
  match scene {
    SceneDefinition::Led(LedScene { targets: LedTargets::Multiple(targets), .. }) => targets.keys().cloned().map(Some).collect(),
    _ => vec![None],
  }
}

/// Some LEDs a scene plays on, each with where it samples them
struct LedPart {
  /// Which layers they show. `None` is all of them.
  target: Option<String>,
  placed: Vec<(String, Sample)>,
  /// The plane's, for a target on one
  size: Option<Vec2>,
}

/// What a scene plays on the LEDs, as `LedPlayback` splits it up. One with no targets yet shows on every LED.
fn led_parts(scene: &SceneDefinition, surface: &LedSurface) -> Vec<LedPart> {
  let SceneDefinition::Led(scene) = scene else {
    return Vec::new();
  };
  let part = |target: Option<&str>, plane: Option<&'static ReferencePlane>, query: &LedQ| {
    let placed = place_leds(scene.viewport, &surface.leds, plane, query, surface.radius);
    let placed = placed.into_iter().map(|(led, at)| (led.name.clone(), at)).collect();
    LedPart { target: target.map(str::to_string), placed, size: plane.map(|plane| plane.extent) }
  };

  if matches!(scene.targets, LedTargets::None) {
    return vec![part(None, surface.plane, &LedQ::Every)];
  }
  let plane_of = |name: &Option<String>| name.as_deref().and_then(|plane| surface.leds.plane(plane));
  scene.targets.iter().map(|(name, target)| part(name, plane_of(&target.plane), &target.leds)).collect()
}

/// What each LED shows while the scene plays on them alone. Where parts overlap, the later one is on top.
fn led_colors(parts: &[LedPart], composition: &GfxComposition, frame: &Frame) -> BTreeMap<String, Rgb> {
  let mut shown: BTreeMap<String, Rgba<u8>> = BTreeMap::new();
  for part in parts {
    let frame = Frame { target: part.target.as_deref(), size: part.size.unwrap_or(frame.size), ..*frame };
    for (name, at) in &part.placed {
      let color = composition.sample(at, &frame);
      shown.entry(name.clone()).and_modify(|below| *below = color.composite_over(*below)).or_insert(color);
    }
  }
  shown.into_iter().map(|(name, color)| (name, led_color(color))).collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use frontbox_core::prelude::{DEFAULT_LED_RADIUS, ExpAddress, LED, LedAddress, Vec3};
  use frontbox_dmd::DmdScene;
  use std::sync::LazyLock;

  const RED_TO_BLUE: &str = r#"Led(
    version: 1, viewport: Sequence, composition: (
    layers: [
      (fill: (base: Gradient(stops: [
        (at: 0.0, color: Fixed(255, 0, 0, 255)),
        (at: 1.0, color: Fixed(0, 0, 255, 255)),
      ]))),
    ],
  ))"#;

  const SIZE: SceneSize = SceneSize { leds: 3, width: 4, height: 2 };

  fn ron(source: &str) -> SceneSource {
    SceneSource::Ron(source.into())
  }

  fn no_leds() -> LedSurface {
    LedSurface { leds: LedLookup::default(), plane: None, radius: DEFAULT_LED_RADIUS }
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
  const AREA: &str = r#"Led(
    version: 1, viewport: Area, composition: (
    layers: [
      (fill: (base: Gradient(stops: [
        (at: 0.0, color: Fixed(255, 0, 0, 255)),
        (at: 1.0, color: Fixed(0, 0, 255, 255)),
      ]))),
    ],
  ))"#;

  fn colors(source: &str, target: LedSurface) -> BTreeMap<String, Rgb> {
    render_scene(ron(source), SIZE, target, None, 0.0, None).unwrap().led_colors
  }

  #[test]
  fn a_sequence_spreads_over_the_leds_in_order() {
    let target = LedSurface { leds: leds(&[None, None, None]), ..no_leds() };
    let colors = colors(RED_TO_BLUE, target);
    assert_eq!(colors["led0"], Rgb::new(255, 0, 0));
    assert_eq!(colors["led2"], Rgb::new(0, 0, 255));
  }

  #[test]
  fn a_sequence_spreads_over_only_the_chosen_leds_in_their_order() {
    let source = RED_TO_BLUE.replace("viewport: Sequence,", r#"viewport: Sequence, targets: Single(leds: Names(["led2", "led0"])),"#);
    let target = LedSurface { leds: leds(&[None, None, None]), ..no_leds() };
    let colors = colors(&source, target);
    assert_eq!(colors.len(), 2);
    assert_eq!(colors["led2"], Rgb::new(255, 0, 0));
    assert_eq!(colors["led0"], Rgb::new(0, 0, 255));
  }

  /// Red on `ring` and blue on `center`, for a second
  fn two_targets(ring: &[&str], center: &[&str]) -> String {
    format!(
      r#"Led(
        version: 1,
        viewport: Sequence,
        targets: Multiple({{"ring": (leds: Names({ring:?})), "center": (leds: Names({center:?}))}}),
        composition: (
          timing: (duration: 1.0),
          layers: [
            (targets: ["ring"], fill: (base: Solid(Fixed(255, 0, 0, 255)))),
            (targets: ["center"], fill: (base: Solid(Fixed(0, 0, 255, 255)))),
          ],
        ),
      )"#
    )
  }

  fn three() -> LedSurface {
    LedSurface { leds: leds(&[None, None, None]), ..no_leds() }
  }

  #[test]
  fn every_target_plays_on_its_own_leds_at_once() {
    let colors = colors(&two_targets(&["led0", "led1"], &["led2"]), three());
    assert_eq!(colors["led0"], Rgb::new(255, 0, 0));
    assert_eq!(colors["led1"], Rgb::new(255, 0, 0));
    assert_eq!(colors["led2"], Rgb::new(0, 0, 255));
  }

  #[test]
  fn the_later_target_is_on_top_where_they_overlap() {
    let colors = colors(&two_targets(&["led0"], &["led0"]), three());
    assert_eq!(colors["led0"], Rgb::new(0, 0, 255));
  }

  #[test]
  fn each_target_gets_a_row_of_the_strip() {
    let rendered = render_scene(ron(&two_targets(&[], &[])), SIZE, no_leds(), None, 0.0, None).unwrap();
    assert_eq!((rendered.width, rendered.height), (3, 2));
    assert_eq!(rendered.colors[..4], [255, 0, 0, 255]);
    assert_eq!(rendered.colors[12..16], [0, 0, 255, 255]);
  }

  #[test]
  fn a_clip_plays_every_target_on_one_timeline() {
    let clip = render_clip(1, ron(&two_targets(&["led0"], &["led2"])), SIZE, three(), None, 0).unwrap();
    let header_length = u32::from_le_bytes(clip[..4].try_into().unwrap()) as usize;
    let header: serde_json::Value = serde_json::from_slice(&clip[4..4 + header_length]).unwrap();
    assert_eq!(header["leds"], serde_json::json!(["led0", "led2"]));
    assert_eq!(header["height"], 2);
    assert_eq!(clip[clip.len() - 6..], [255, 0, 0, 0, 0, 255]);
  }

  #[test]
  fn an_area_colors_leds_by_where_they_are_on_the_plane() {
    let at = |x: f32| Some(Vec3::new(100.0 + x, 5.0, 0.0));
    let target = LedSurface { leds: leds(&[at(10.0), at(0.0)]), plane: Some(&PLANE), ..no_leds() };
    let colors = colors(AREA, target);
    assert_eq!(colors["led0"], Rgb::new(0, 0, 255));
    assert_eq!(colors["led1"], Rgb::new(255, 0, 0));
  }

  #[test]
  fn leds_past_the_edge_of_the_plane_show_the_shapes_that_reach_them() {
    let source = r#"Led(
      version: 1, viewport: Area, composition: (
      layers: [
        (fill: (base: Solid(Fixed(0, 0, 255, 255)))),
        (
          shape: (base: Ellipse(r1: Fixed(0.5), r2: Fixed(0.5)), transform: [Scale(Fixed(4.0, 4.0)), Translate(Fixed(15.0, 5.0))]),
          fill: (base: Solid(Fixed(255, 0, 0, 255))),
        ),
      ],
    ))"#;
    let at = |x: f32| Some(Vec3::new(x, 5.0, 0.0));
    let target = LedSurface { leds: leds(&[at(15.0), at(25.0)]), plane: Some(&SQUARE), ..no_leds() };
    let colors = colors(source, target);
    assert_eq!(colors["led0"], Rgb::new(255, 0, 0));
    assert_eq!(colors["led1"], Rgb::new(0, 0, 0));
  }

  #[test]
  fn shape_edges_on_leds_fade_across_the_radius_the_machine_was_booted_with() {
    let source = r#"Led(
      version: 1, viewport: Area, composition: (
      layers: [(
        shape: (base: Ellipse(r1: Fixed(0.5), r2: Fixed(0.5)), transform: [Scale(Fixed(4.0, 4.0)), Translate(Fixed(5.0, 5.0))]),
        fill: (base: Solid(Fixed(255, 0, 0, 255))),
      )],
    ))"#;
    let brightness = |radius| {
      let target = LedSurface {
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
    let source = "Led(version: 1, viewport: Area, composition: (layers: []))";
    let rendered = render_scene(ron(source), SIZE, no_leds(), None, 0.0, None).unwrap();
    assert_eq!((rendered.width, rendered.height), (4, 2));
  }

  #[test]
  fn an_area_says_what_a_render_covers() {
    // 4 x 2 is twice as wide as this square plane, so half of it falls either side
    let target = LedSurface { plane: Some(&SQUARE), ..no_leds() };
    let source = "Led(version: 1, viewport: Area, composition: (layers: []))";
    let area = render_scene(ron(source), SIZE, target, None, 0.0, None).unwrap().area;
    assert_eq!((area.x, area.y, area.width, area.height), (-5.0, 0.0, 20.0, 10.0));
  }

  #[test]
  fn an_area_with_no_plane_is_the_size_of_the_render() {
    let source = "Led(version: 1, viewport: Area, composition: (layers: []))";
    let area = render_scene(ron(source), SIZE, no_leds(), None, 0.0, None).unwrap().area;
    assert_eq!((area.x, area.y, area.width, area.height), (0.0, 0.0, 4.0, 2.0));
  }

  #[test]
  fn a_layer_with_no_shape_outlines_the_plane() {
    let target = LedSurface { plane: Some(&SQUARE), ..no_leds() };
    let source = "Led(version: 1, viewport: Area, composition: (layers: [(fill: (base: Solid(Fixed(1, 2, 3, 255))))]))";
    let outlines = render_scene(ron(source), SIZE, target, None, 0.0, None).unwrap().outlines;
    assert_eq!(outlines, [[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]]);
  }

  #[test]
  fn a_shape_outlines_where_it_is_on_the_plane() {
    let target = LedSurface { plane: Some(&SQUARE), ..no_leds() };
    let source = r#"Led(
      version: 1, viewport: Area, composition: (
      layers: [(
        shape: (base: Polygon(points: [Fixed(0.0, 0.0), Fixed(1.0, 0.0), Fixed(0.0, 1.0)]), transform: [Translate(Fixed(2.0, 3.0))]),
        fill: (base: Solid(Fixed(1, 2, 3, 255))),
      )],
    ))"#;
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
    let source = r#"Led(
      version: 1, viewport: Sequence, composition: (
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
    ))"#;
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
    let source = r#"Led(
      version: 1, viewport: Area, composition: (
      layers: [(
        shape: (base: Ellipse(r1: Fixed(0.5), r2: Fixed(0.5)), transform: [Scale(Fixed(40.0, 40.0)), Translate(Fixed(50.0, 25.0))]),
        fill: (base: Solid(Fixed(255, 0, 0, 255))),
      )],
    ))"#;
    let target = LedSurface { plane: Some(&WIDE), ..no_leds() };
    let rendered = render_scene(ron(source), SceneSize { leds: 0, width: 3, height: 3 }, target, None, 0.0, None).unwrap();
    let pixel = |x: usize, y: usize| &rendered.colors[(y * 3 + x) * 4..][..4];
    assert_eq!(pixel(1, 1), [255, 0, 0, 255]);
    assert_eq!(pixel(0, 0), [0, 0, 0, 0]);
  }

  #[test]
  fn reports_where_a_scene_is_wrong() {
    let error = render_scene(ron("Led(version: 1, viewport: Sequence, composition: (layers: [(fill: (base: Nope))]))"), SIZE, no_leds(), None, 0.0, None);
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

  fn solid(color: &str, timing: &str) -> SceneDefinition {
    let source = format!("Led(version: 1, viewport: Sequence, composition: ({timing} layers: [(fill: (base: Solid(Fixed({color}))))]))");
    render_scene(ron(&source), SIZE, no_leds(), None, 0.0, None).unwrap().scene
  }

  #[test]
  fn a_preview_draws_what_it_shows_and_gives_back_the_scene() {
    let (scene, shown) = (solid("255, 0, 0, 255", ""), solid("0, 0, 255, 255", ""));
    let from_definition = render_scene(SceneSource::Definition(scene.clone()), SIZE, no_leds(), None, 0.0, None).unwrap();
    let preview = render_scene(SceneSource::Preview { scene, shown }, SIZE, no_leds(), None, 0.0, None).unwrap();
    assert_eq!(preview.colors[0..3], [0, 0, 255]);
    assert_eq!(preview.ron, from_definition.ron);
  }

  #[test]
  fn a_clip_of_a_preview_is_of_what_it_shows() {
    let timing = "timing: (duration: 1.0),";
    let source = SceneSource::Preview { scene: solid("255, 0, 0, 255", timing), shown: solid("0, 0, 255, 255", timing) };
    let clip = render_clip(0, source, SIZE, no_leds(), None, 0).unwrap();
    let header = u32::from_le_bytes(clip[0..4].try_into().unwrap()) as usize;
    assert_eq!(clip[4 + header..7 + header], [0, 0, 255]);
  }

  #[test]
  fn what_a_client_keeps_on_a_track_for_itself_doesnt_reach_the_file() {
    let source = r#"Led(
      version: 1, viewport: Sequence, composition: (
      timing: (duration: 1.0),
      layers: [(fill: (base: Solid(Track(keys: [(at: 0.0, value: (9, 8, 7, 255))]))))],
    ))"#;
    let scene = render_scene(ron(source), SIZE, no_leds(), None, 0.0, None).unwrap().scene;
    let mut json = serde_json::to_value(&scene).unwrap();
    json["Led"]["composition"]["layers"][0]["fill"]["base"]["Solid"]["Track"]["hidden"] = true.into();
    let back = render_scene(SceneSource::Definition(serde_json::from_value(json).unwrap()), SIZE, no_leds(), None, 0.0, None);
    let back = back.unwrap();
    assert!(!back.ron.contains("hidden"), "{}", back.ron);
  }

  #[test]
  fn a_definition_is_held_to_the_same_rules_as_a_file() {
    let source = r#"Led(
      version: 1, viewport: Area, composition: (
      timing: (duration: 1.0),
      layers: [(fill: (base: Solid(Track(keys: [(at: 0.0, value: (0, 0, 0, 255))]))))],
    ))"#;
    let SceneDefinition::Led(mut scene) = render_scene(ron(source), SIZE, no_leds(), None, 0.0, None).unwrap().scene else {
      panic!("not an LED scene");
    };
    scene.composition.timing = None;
    let scene = SceneDefinition::Led(scene);
    let error = render_scene(SceneSource::Definition(scene), SIZE, no_leds(), None, 0.0, None).err().unwrap();
    assert!(error.contains("timing"), "{error}");
  }

  /// Goes from black to red over two seconds
  fn rising(cycle: &str) -> SceneSource {
    ron(&format!(
      r#"Led(
        version: 1, viewport: Sequence, composition: (
        timing: (duration: 2.0, cycle: {cycle}),
        layers: [(fill: (base: Solid(Track(keys: [
          (at: 0.0, value: (0, 0, 0, 255)),
          (at: 2.0, value: (200, 0, 0, 255)),
        ]))))],
      ))"#
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
        r#"Led(
          version: 1, viewport: Sequence, composition: (
          timing: (duration: 1.0, cycle: {cycle}),
          layers: [(fill: (base: Solid(Random(
            picks: [(at: 0.0, smooth_in: 1.0)],
            spread: Color(hue: (0.0, 360.0), brightness: (1.0, 1.0)),
          ))))],
        ))"#
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
      r#"Led(
        version: 1, viewport: Sequence, composition: (
        timing: (duration: 1.0, cycle: {cycle}),
        layers: [(fill: (base: Solid(Random(
          picks: [(at: 0.0, smooth_in: {smooth_in})],
          spread: Color(hue: (0.0, 360.0), brightness: (1.0, 1.0)),
        ))))],
      ))"#
    ))
  }

  #[test]
  fn a_scene_is_filed_as_what_was_typed_or_the_definition_being_kept() {
    let typed = "// the title\nLed(version: 1, viewport: Area, composition: (layers: []))";
    assert_eq!(scene_file(SceneSource::Ron(typed.into())).unwrap(), typed);

    let scene = SceneDefinition::load(typed).unwrap();
    let shown = SceneDefinition::Dmd(DmdScene::new(GfxComposition::default()));
    let written = scene.save().unwrap();
    assert_eq!(scene_file(SceneSource::Preview { scene, shown }).unwrap(), written);
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

  const IMAGE: &str = r#"Led(version: 1, viewport: Area, composition: ( layers: [(fill: (base: Image(name: "art/a.png")))]))"#;
  const TWO_POINTS: SceneSize = SceneSize { leds: 0, width: 2, height: 1 };

  #[test]
  fn a_render_shows_a_mask_and_says_where_it_is() {
    let source = r#"Led(
      version: 1, viewport: Area, composition: (
      layers: [
        (fill: (base: Solid(Fixed(0, 0, 255, 255)))),
        (
          fill: (base: Solid(Fixed(255, 0, 0, 255))),
          mask: (
            fill: (base: Solid(Fixed(0, 0, 0, 255))),
            shape: (base: Ellipse(r1: Fixed(0.5), r2: Fixed(0.5)), transform: [Scale(Fixed(40.0, 40.0)), Translate(Fixed(50.0, 25.0))]),
          ),
        ),
      ],
    ))"#;
    let target = LedSurface { plane: Some(&WIDE), ..no_leds() };
    let rendered = render_scene(ron(source), SceneSize { leds: 0, width: 10, height: 5 }, target, None, 0.0, None).unwrap();
    let pixel = |x: usize, y: usize| &rendered.colors[(y * 10 + x) * 4..][..4];
    assert_eq!(pixel(5, 2), [255, 0, 0, 255]);
    assert_eq!(pixel(0, 2), [0, 0, 255, 255]);
    assert!(rendered.mask_shapes[0].is_none());
    assert!(rendered.mask_shapes[1].is_some());
    assert!(rendered.mask_outlines[0].is_empty());
    assert!(!rendered.mask_outlines[1].is_empty());
  }

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
    let source = r#"Led(version: 1, viewport: Area, composition: ( timing: (duration: 0.1, cycle: Once), layers: [(fill: (base: Image(name: "art/a.png")))]))"#;
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

    let source = r#"Led(version: 1, viewport: Area, composition: ( timing: (duration: 1.0, cycle: Once), layers: [(fill: (base: Image(
      name: "a.gif",
      frame: Track(keys: [(at: 0.0, value: 0.0), (at: 1.0, value: 2.0)]),
    )))]))"#;
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
    let source = r#"Led(
      version: 1, viewport: Sequence, composition: (
      timing: (duration: 0.1),
      layers: [(fill: (base: Solid(Fixed(200, 0, 0, 128))))],
    ))"#;
    let target = LedSurface { leds: leds(&[None, None]), ..no_leds() };
    let clip = render_clip(0, ron(source), SIZE, target, None, 0).unwrap();
    let (header, colors) = read_clip(&clip);
    assert_eq!(header["leds"], serde_json::json!(["led0", "led1"]));

    let frames = header["frames"].as_u64().unwrap() as usize;
    let leds = &colors[frames * 3 * 4..];
    assert_eq!(leds.len(), frames * 2 * 3);
    assert_eq!(leds[..6], [100, 0, 0, 100, 0, 0]);
  }

  #[test]
  fn a_target_says_which_plane_its_on() {
    let source = AREA.replace("viewport: Area,", r#"viewport: Area, targets: Single(leds: Every, plane: "Square"),"#);
    let mut lookup = leds(&[Some(Vec3::new(10.0, 5.0, 0.0))]);
    lookup.add_plane(&SQUARE);
    // not the plane the editor has chosen, which is for scenes that haven't said
    let target = LedSurface { leds: lookup, plane: Some(&WIDE), ..no_leds() };
    let rendered = render_scene(ron(&source), SIZE, target, None, 0.0, None).unwrap();
    assert_eq!(rendered.led_colors["led0"], Rgb::new(0, 0, 255));
    assert_eq!((rendered.area.width, rendered.area.height), (20.0, 10.0));
  }

  #[test]
  fn a_dmd_scene_is_its_own_device_and_reaches_no_leds() {
    let source = "Dmd(version: 1, composition: (layers: [(fill: (base: Solid(Fixed(255, 0, 0, 255))))]))";
    let target = LedSurface { leds: leds(&[None]), plane: Some(&SQUARE), ..no_leds() };
    let rendered = render_scene(ron(source), SIZE, target, None, 0.0, None).unwrap();
    assert_eq!((rendered.width, rendered.height), (4, 2));
    assert_eq!((rendered.area.width, rendered.area.height), (4.0, 2.0));
    assert!(rendered.led_colors.is_empty());
    assert!(rendered.ron.starts_with("Dmd("), "{}", rendered.ron);
  }

  #[test]
  fn a_scene_that_cant_be_played_isnt_filed() {
    assert!(scene_file(SceneSource::Ron("(oops".into())).is_err());
    let source = RED_TO_BLUE.replace("viewport: Sequence,", r#"viewport: Area, targets: Single(leds: Every),"#);
    assert!(scene_file(SceneSource::Ron(source)).is_err());
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
    let source = r#"Led(
      version: 1, viewport: Area, composition: (params: {"tint": Color(1, 2, 3, 4), "size": Float(2.5), "at": Vec2(0.5, 0.25)},
      
      timing: (duration: 2.0, cycle: Forever),
      layers: [(
        shape: (
          base: Polygon(points: [Fixed(0.0, 0.0), Param("at"), Fixed(1.0, 0.0)]),
          transform: [Rotate(Track(keys: [(at: 0.0, value: 0.0), (at: 1.0, value: 90.0, curve: Steps(4))], cycle: Times(2)))],
        ),
        fill: (base: Solid(Param("tint")), adjust: [HueShift(Param("size"))]),
      )],
    ))"#;
    let rendered = render_scene(ron(source), SIZE, no_leds(), None, 0.0, None).unwrap();
    let json = serde_json::to_string(&rendered.scene).unwrap();
    let back = render_scene(SceneSource::Definition(serde_json::from_str(&json).unwrap()), SIZE, no_leds(), None, 0.0, None).unwrap();
    assert_eq!(back.ron, rendered.ron);
  }

  #[test]
  fn a_sweep_survives_the_editor_and_only_on_a_position() {
    let source = r#"Led(
      version: 1, viewport: Area, composition: (
      timing: (duration: 2.0, cycle: Forever),
      layers: [(
        shape: (
          base: Ellipse(r1: Fixed(1.0), r2: Fixed(1.0)),
          transform: [Translate(Sweep(path: (base: Rect(width: Fixed(4.0), height: Fixed(2.0)), transform: [Translate(Fixed(5.0, 5.0))]), start: 0.5, end: 1.5))],
        ),
        fill: (base: Solid(Fixed(255, 0, 0, 255))),
      )],
    ))"#;
    let rendered = render_scene(ron(source), SIZE, no_leds(), None, 0.0, None).unwrap();
    let json = serde_json::to_string(&rendered.scene).unwrap();
    let back = render_scene(SceneSource::Definition(serde_json::from_str(&json).unwrap()), SIZE, no_leds(), None, 0.0, None).unwrap();
    assert_eq!(back.ron, rendered.ron);

    let path = rendered.shapes[0].as_ref().unwrap().paths[0].as_ref().unwrap();
    assert_eq!(path.outline[0], [3.0, 4.0]);

    let on_a_size = json.replace(r#""r1":{"Fixed":1.0}"#, &format!(r#""r1":{}"#, sweep_of(&json)));
    let error = serde_json::from_str::<SceneDefinition>(&on_a_size).unwrap_err().to_string();
    assert!(error.contains("only a position"), "{error}");
  }

  /// The first sweep in a scene as JSON
  fn sweep_of(json: &str) -> String {
    let scene: serde_json::Value = serde_json::from_str(json).unwrap();
    scene["Led"]["composition"]["layers"][0]["shape"]["transform"][0]["Translate"].to_string()
  }
}
