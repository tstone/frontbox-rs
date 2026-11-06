use frontbox_gfx::layout::{Grid, Line};
use frontbox_gfx::scene::{Bounds, Frame, Params, Sample, Scene, file};

use crate::protocol::{RenderedScene, SceneTarget};

/// The most points one render will sample. A render costs the machine time and anything on the network can ask for
/// one. This is far more than any DMD or list of LEDs.
const MAX_SAMPLES: usize = 1 << 18;

/// Renders a scene from its RON source, at its start and with its default parameters. The error is for showing to
/// whoever wrote the scene.
pub(crate) fn render_scene(source: &str, target: SceneTarget) -> Result<RenderedScene, String> {
  let scene = file::load(source).map_err(|err| err.to_string())?;

  let (width, height) = match target {
    SceneTarget::Line { count } => (count, 1),
    SceneTarget::Grid { width, height } => (width, height),
  };
  if width.saturating_mul(height) > MAX_SAMPLES {
    return Err(format!(
      "{width} x {height} is more than the {MAX_SAMPLES} points a render can sample"
    ));
  }

  let colors = match target {
    SceneTarget::Line { count } => {
      let line = Line::new(count);
      sample(&scene, line.samples(), line.stage())
    }
    SceneTarget::Grid { width, height } => {
      let grid = Grid::new(width, height);
      let grid = scene.size.map_or(grid, |size| grid.over(Bounds::sized(size)));
      sample(&scene, grid.samples(), grid.stage())
    }
  };

  Ok(RenderedScene { width, height, colors })
}

fn sample(scene: &Scene, samples: impl Iterator<Item = Sample>, stage: Bounds) -> Vec<u8> {
  let params = Params::new();
  let frame = Frame { time: 0.0, stage, params: &params };
  samples.flat_map(|at| scene.sample(&at, &frame).0).collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  const RED_TO_BLUE: &str = r#"(
    version: 1,
    layers: [
      (paint: (base: Gradient(stops: [
        (at: 0.0, color: Fixed(255, 0, 0, 255)),
        (at: 1.0, color: Fixed(0, 0, 255, 255)),
      ]))),
    ],
  )"#;

  #[test]
  fn line_runs_end_to_end() {
    let rendered = render_scene(RED_TO_BLUE, SceneTarget::Line { count: 3 }).unwrap();
    assert_eq!((rendered.width, rendered.height), (3, 1));
    assert_eq!(rendered.colors.len(), 12);
    assert_eq!(rendered.colors[..4], [255, 0, 0, 255]);
    assert_eq!(rendered.colors[8..], [0, 0, 255, 255]);
  }

  #[test]
  fn grid_covers_a_sized_stage() {
    // a circle in the center of a 100 x 50 stage, at far less resolution than the stage has units
    let source = r#"(
      version: 1,
      size: (100.0, 50.0),
      layers: [(
        shape: (base: Ellipse, warp: [Scale(Fixed(40.0, 40.0)), Translate(Fixed(50.0, 25.0))]),
        paint: (base: Solid(Fixed(255, 0, 0, 255))),
      )],
    )"#;
    let rendered = render_scene(source, SceneTarget::Grid { width: 3, height: 3 }).unwrap();
    let pixel = |x: usize, y: usize| &rendered.colors[(y * 3 + x) * 4..][..4];
    assert_eq!(pixel(1, 1), [255, 0, 0, 255]);
    assert_eq!(pixel(0, 0), [0, 0, 0, 0]);
  }

  #[test]
  fn reports_where_a_scene_is_wrong() {
    let error = render_scene(
      "(version: 1, layers: [(paint: (base: Nope))])",
      SceneTarget::Line { count: 3 },
    );
    assert!(error.err().unwrap().contains("1:"));
  }

  #[test]
  fn refuses_a_target_too_large_to_sample() {
    let target = SceneTarget::Grid { width: usize::MAX, height: 2 };
    assert!(render_scene(RED_TO_BLUE, target).is_err());
  }
}
