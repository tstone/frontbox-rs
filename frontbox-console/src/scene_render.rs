use frontbox_core::prelude::{LED, LedLookup, LedQ, ReferencePlane, Rgb, Vec2};
use frontbox_gfx::layout::{Grid, Line};
use frontbox_gfx::prelude::{Rgba, RgbaColor};
use frontbox_gfx::scene::{Frame, Params, Sample, Scene, Viewport, file};
use frontbox_led::{DEFAULT_LED_RADIUS, place_on_plane};
use std::collections::BTreeMap;

use crate::protocol::{RenderedArea, RenderedScene, SceneSize, SceneSource};

/// The most points one render will sample. A render costs the machine time and anything on the network can ask for
/// one. This is far more than any DMD or list of LEDs.
const MAX_SAMPLES: usize = 1 << 18;

/// The LEDs to play a scene on, mirroring how `LedScene` is given them
pub(crate) struct LedTarget {
  pub leds: LedLookup,
  /// What an `Area` scene plays on. A `Sequence` scene doesn't use one.
  pub plane: Option<&'static ReferencePlane>,
  pub query: LedQ,
}

/// Renders a scene at its start and with its default parameters, to whichever of the sizes suits its viewport. The
/// reply has the scene in both forms, as this is the only place RON is read or written for a client. The error is for
/// showing to whoever wrote the scene.
pub(crate) fn render_scene(
  source: SceneSource,
  size: SceneSize,
  target: LedTarget,
) -> Result<RenderedScene, String> {
  let scene = match source {
    SceneSource::Ron(ron) => file::load(&ron).map_err(|err| err.to_string())?,
    SceneSource::Definition(scene) => scene,
  };

  let (width, height) = match scene.viewport {
    Viewport::Sequence => (size.leds, 1),
    Viewport::Area => (size.width, size.height),
  };
  if width.saturating_mul(height) > MAX_SAMPLES {
    return Err(format!(
      "{width} x {height} is more than the {MAX_SAMPLES} points a render can sample"
    ));
  }

  // what the device playing the scene would give it
  let device_size = target.plane.map_or(Vec2::new(width as f32, height as f32), |plane| plane.extent);
  let params = Params::new();
  let frame = Frame { time: 0.0, params: &params, size: device_size };

  let (colors, area, outlines) = match scene.viewport {
    Viewport::Sequence => (sample(&scene, Line::new(width).samples(), &frame), None, Vec::new()),
    Viewport::Area => {
      let grid = Grid::new(width, height, device_size);
      let outlines = scene.outlines(&frame);
      let outlines = outlines.iter().map(|outline| outline.iter().map(|p| p.to_array()).collect()).collect();
      (sample(&scene, grid.samples(), &frame), Some(rendered_area(&grid)), outlines)
    }
  };
  let led_colors = led_colors(&scene, &target, &frame);
  let ron = file::save(&scene).map_err(|err| err.to_string())?;

  Ok(RenderedScene { scene, ron, width, height, colors, area, outlines, led_colors })
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
  let placed = placed(scene.viewport, target);
  let colors = sample(scene, placed.iter().map(|(_, at)| at.clone()), frame);
  placed
    .iter()
    .zip(colors.chunks_exact(4))
    .map(|((led, _), color)| {
      let [r, g, b, _] = Rgba([color[0], color[1], color[2], color[3]]).composite_over(Rgba::black()).0;
      (led.name.clone(), Rgb::new(r, g, b))
    })
    .collect()
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
    render_scene(ron(source), SIZE, target).unwrap().led_colors
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
    let rendered = render_scene(ron(RED_TO_BLUE), SIZE, no_leds()).unwrap();
    assert_eq!((rendered.width, rendered.height), (3, 1));
    assert_eq!(rendered.colors.len(), 12);
    assert_eq!(rendered.colors[..4], [255, 0, 0, 255]);
    assert_eq!(rendered.colors[8..], [0, 0, 255, 255]);
  }

  #[test]
  fn an_area_renders_to_pixels() {
    let source = "(version: 1, viewport: Area, layers: [])";
    let rendered = render_scene(ron(source), SIZE, no_leds()).unwrap();
    assert_eq!((rendered.width, rendered.height), (4, 2));
  }

  #[test]
  fn an_area_says_what_a_render_covers() {
    // 4 x 2 is twice as wide as this square plane, so half of it falls either side
    let target = LedTarget { plane: Some(&SQUARE), ..no_leds() };
    let source = "(version: 1, viewport: Area, layers: [])";
    let area = render_scene(ron(source), SIZE, target).unwrap().area.unwrap();
    assert_eq!((area.x, area.y, area.width, area.height), (-5.0, 0.0, 20.0, 10.0));
  }

  #[test]
  fn an_area_with_no_plane_is_the_size_of_the_render() {
    let source = "(version: 1, viewport: Area, layers: [])";
    let area = render_scene(ron(source), SIZE, no_leds()).unwrap().area.unwrap();
    assert_eq!((area.x, area.y, area.width, area.height), (0.0, 0.0, 4.0, 2.0));
  }

  #[test]
  fn a_layer_with_no_shape_outlines_the_plane() {
    let target = LedTarget { plane: Some(&SQUARE), ..no_leds() };
    let source = "(version: 1, viewport: Area, layers: [(paint: (base: Solid(Fixed(1, 2, 3, 255))))])";
    let outlines = render_scene(ron(source), SIZE, target).unwrap().outlines;
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
    let outlines = render_scene(ron(source), SIZE, target).unwrap().outlines;
    assert_eq!(outlines, [[[2.0, 3.0], [3.0, 3.0], [2.0, 4.0]]]);
  }

  #[test]
  fn a_sequence_has_no_outlines() {
    assert!(render_scene(ron(RED_TO_BLUE), SIZE, no_leds()).unwrap().outlines.is_empty());
  }

  #[test]
  fn a_sequence_has_no_area() {
    assert!(render_scene(ron(RED_TO_BLUE), SIZE, no_leds()).unwrap().area.is_none());
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
    let rendered = render_scene(ron(source), SceneSize { leds: 0, width: 3, height: 3 }, target).unwrap();
    let pixel = |x: usize, y: usize| &rendered.colors[(y * 3 + x) * 4..][..4];
    assert_eq!(pixel(1, 1), [255, 0, 0, 255]);
    assert_eq!(pixel(0, 0), [0, 0, 0, 0]);
  }

  #[test]
  fn reports_where_a_scene_is_wrong() {
    let error = render_scene(ron("(version: 1, viewport: Sequence, layers: [(paint: (base: Nope))])"), SIZE, no_leds());
    assert!(error.err().unwrap().contains("1:"));
  }

  #[test]
  fn refuses_a_size_too_large_to_sample() {
    let size = SceneSize { leds: usize::MAX, ..SIZE };
    assert!(render_scene(ron(RED_TO_BLUE), size, no_leds()).is_err());
  }

  #[test]
  fn a_definition_renders_like_its_ron() {
    let from_ron = render_scene(ron(RED_TO_BLUE), SIZE, no_leds()).unwrap();
    let from_definition = render_scene(SceneSource::Definition(from_ron.scene), SIZE, no_leds()).unwrap();
    assert_eq!(from_definition.colors, from_ron.colors);
    assert_eq!(from_definition.ron, from_ron.ron);
  }

  #[test]
  fn a_scene_survives_the_trip_to_a_client_and_back() {
    let source = r#"(
      version: 1,
      params: {"tint": Color(1, 2, 3, 4), "size": Float(2.5), "at": Vec2(0.5, 0.25)},
      viewport: Area,
      layers: [(
        shape: (
          base: Polygon(points: [Fixed(0.0, 0.0), Param("at"), Fixed(1.0, 0.0)]),
          warp: [Rotate(Track(keys: [(at: 0.0, value: 0.0, curve: Steps(4)), (at: 1.0, value: 90.0)], cycle: Times(2)))],
        ),
        paint: (base: Solid(Param("tint")), adjust: [HueShift(Param("size"))]),
      )],
    )"#;
    let rendered = render_scene(ron(source), SIZE, no_leds()).unwrap();
    let json = serde_json::to_string(&rendered.scene).unwrap();
    let back = render_scene(SceneSource::Definition(serde_json::from_str(&json).unwrap()), SIZE, no_leds()).unwrap();
    assert_eq!(back.ron, rendered.ron);
  }
}
