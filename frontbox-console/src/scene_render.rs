use frontbox_gfx::layout::{Grid, Line};
use frontbox_gfx::scene::{Frame, Params, Sample, Scene, Viewport, file};

use crate::protocol::{RenderedScene, SceneSize, SceneSource};

/// The most points one render will sample. A render costs the machine time and anything on the network can ask for
/// one. This is far more than any DMD or list of LEDs.
const MAX_SAMPLES: usize = 1 << 18;

/// Renders a scene at its start and with its default parameters, to whichever of the sizes suits its viewport. The
/// reply has the scene in both forms, as this is the only place RON is read or written for a client. The error is for
/// showing to whoever wrote the scene.
pub(crate) fn render_scene(source: SceneSource, size: SceneSize) -> Result<RenderedScene, String> {
  let scene = match source {
    SceneSource::Ron(ron) => file::load(&ron).map_err(|err| err.to_string())?,
    SceneSource::Definition(scene) => scene,
  };

  let (width, height) = match scene.viewport {
    Viewport::Sequence => (size.leds, 1),
    Viewport::Area(_) => (size.width, size.height),
  };
  if width.saturating_mul(height) > MAX_SAMPLES {
    return Err(format!(
      "{width} x {height} is more than the {MAX_SAMPLES} points a render can sample"
    ));
  }

  let colors = match scene.viewport {
    Viewport::Sequence => sample(&scene, Line::new(width).samples()),
    Viewport::Area(area) => sample(&scene, Grid::new(width, height, area).samples()),
  };
  let ron = file::save(&scene).map_err(|err| err.to_string())?;

  Ok(RenderedScene { scene, ron, width, height, colors })
}

fn sample(scene: &Scene, samples: impl Iterator<Item = Sample>) -> Vec<u8> {
  let params = Params::new();
  let frame = Frame { time: 0.0, params: &params };
  samples.flat_map(|at| scene.sample(&at, &frame).0).collect()
}

#[cfg(test)]
mod tests {
  use super::*;

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

  #[test]
  fn a_sequence_renders_to_leds() {
    let rendered = render_scene(ron(RED_TO_BLUE), SIZE).unwrap();
    assert_eq!((rendered.width, rendered.height), (3, 1));
    assert_eq!(rendered.colors.len(), 12);
    assert_eq!(rendered.colors[..4], [255, 0, 0, 255]);
    assert_eq!(rendered.colors[8..], [0, 0, 255, 255]);
  }

  #[test]
  fn an_area_renders_to_pixels() {
    let source = "(version: 1, viewport: Area(100.0, 50.0), layers: [])";
    let rendered = render_scene(ron(source), SIZE).unwrap();
    assert_eq!((rendered.width, rendered.height), (4, 2));
  }

  #[test]
  fn grid_fits_an_area() {
    // a circle in the center of a 100 x 50 viewport, at far less resolution than it has units
    let source = r#"(
      version: 1,
      viewport: Area(100.0, 50.0),
      layers: [(
        shape: (base: Ellipse, warp: [Scale(Fixed(40.0, 40.0)), Translate(Fixed(50.0, 25.0))]),
        paint: (base: Solid(Fixed(255, 0, 0, 255))),
      )],
    )"#;
    let rendered = render_scene(ron(source), SceneSize { leds: 0, width: 3, height: 3 }).unwrap();
    let pixel = |x: usize, y: usize| &rendered.colors[(y * 3 + x) * 4..][..4];
    assert_eq!(pixel(1, 1), [255, 0, 0, 255]);
    assert_eq!(pixel(0, 0), [0, 0, 0, 0]);
  }

  #[test]
  fn reports_where_a_scene_is_wrong() {
    let error = render_scene(ron("(version: 1, viewport: Sequence, layers: [(paint: (base: Nope))])"), SIZE);
    assert!(error.err().unwrap().contains("1:"));
  }

  #[test]
  fn refuses_a_size_too_large_to_sample() {
    let size = SceneSize { leds: usize::MAX, ..SIZE };
    assert!(render_scene(ron(RED_TO_BLUE), size).is_err());
  }

  #[test]
  fn a_definition_renders_like_its_ron() {
    let from_ron = render_scene(ron(RED_TO_BLUE), SIZE).unwrap();
    let from_definition = render_scene(SceneSource::Definition(from_ron.scene), SIZE).unwrap();
    assert_eq!(from_definition.colors, from_ron.colors);
    assert_eq!(from_definition.ron, from_ron.ron);
  }

  #[test]
  fn a_scene_survives_the_trip_to_a_client_and_back() {
    let source = r#"(
      version: 1,
      params: {"tint": Color(1, 2, 3, 4), "size": Float(2.5), "at": Vec2(0.5, 0.25)},
      viewport: Area(100.0, 50.0),
      layers: [(
        shape: (
          base: Polygon(points: [Fixed(0.0, 0.0), Param("at"), Fixed(1.0, 0.0)]),
          warp: [Rotate(Track(keys: [(at: 0.0, value: 0.0, curve: Steps(4)), (at: 1.0, value: 90.0)], cycle: Times(2)))],
        ),
        paint: (base: Solid(Param("tint")), adjust: [HueShift(Param("size"))]),
      )],
    )"#;
    let rendered = render_scene(ron(source), SIZE).unwrap();
    let json = serde_json::to_string(&rendered.scene).unwrap();
    let back = render_scene(SceneSource::Definition(serde_json::from_str(&json).unwrap()), SIZE).unwrap();
    assert_eq!(back.ron, rendered.ron);
  }
}
