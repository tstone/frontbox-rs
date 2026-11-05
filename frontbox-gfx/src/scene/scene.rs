use glam::Vec2;
use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::color::RgbaColor;
use crate::scene::{Env, Frame, Layer, Params, Bounds, Sample, file};

/// A graphic and how it moves over time: layers on a stage, and the parameters the game can change. Ready to be
/// sampled by any device, and written to a file with [`file`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
  pub version: u32,
  /// Parameter names and their defaults
  #[serde(default, skip_serializing_if = "Params::is_empty")]
  pub params: Params,
  /// The stage in real units, or `None` to take the target's size
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub size: Option<Vec2>,
  /// Painted bottom to top, alpha over
  #[serde(default)]
  pub layers: Vec<Layer>,
}

impl Scene {
  /// An unsized scene of the current format version
  pub fn new(layers: Vec<Layer>) -> Self {
    Self { version: file::VERSION, params: Params::new(), size: None, layers }
  }

  /// Sets the stage's size in real units
  pub fn sized(mut self, size: Vec2) -> Self {
    self.size = Some(size);
    self
  }

  /// The color at a sample
  pub fn sample(&self, at: &Sample, frame: &Frame) -> Rgba<u8> {
    let env = Env { time: frame.time, params: frame.params, defaults: &self.params };
    let stage = self.size.map_or(frame.stage, Bounds::sized);
    self
      .layers
      .iter()
      .fold(Rgba([0, 0, 0, 0]), |below, layer| layer.sample(at, stage, &env).composite_over(below))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::layout::Line;
  use crate::scene::{Color, ColorSpace, Paint, PaintNode, Shape, ShapeNode, Stop, Warp};

  const RED: Rgba<u8> = Rgba([255, 0, 0, 255]);
  const BLUE: Rgba<u8> = Rgba([0, 0, 255, 255]);
  const DARK: Rgba<u8> = Rgba([16, 16, 16, 255]);

  fn solid(color: Rgba<u8>) -> Paint {
    Paint::new(PaintNode::Solid(Color(color).into()))
  }

  fn red_to_blue() -> Paint {
    Paint::new(PaintNode::Gradient {
      stops: vec![Stop::new(0.0, Color(RED)), Stop::new(1.0, Color(BLUE))],
      space: ColorSpace::Rgb,
    })
  }

  fn sample(scene: &Scene, x: f32, y: f32, radius: f32) -> Rgba<u8> {
    let params = Params::new();
    let frame = Frame { time: 0.0, stage: Bounds::sized(Vec2::ZERO), params: &params };
    scene.sample(&Sample::new(Vec2::new(x, y), radius), &frame)
  }

  fn line(scene: &Scene, count: usize) -> Vec<Rgba<u8>> {
    let params = Params::new();
    let line = Line::new(count);
    let frame = Frame { time: 0.0, stage: line.stage(), params: &params };
    line.samples().map(|s| scene.sample(&s, &frame)).collect()
  }

  /// A circle of diameter 40 at (50, 25) on a 100 x 50 stage, over a dark background
  fn circle(paint: Paint) -> Scene {
    Scene::new(vec![
      Layer::new(solid(DARK)),
      Layer::new(paint).shape(
        Shape::new(ShapeNode::Ellipse)
          .warp(Warp::Scale(Vec2::splat(40.0).into()))
          .warp(Warp::Translate(Vec2::new(50.0, 25.0).into())),
      ),
    ])
    .sized(Vec2::new(100.0, 50.0))
  }

  #[test]
  fn solid_on_a_line() {
    let scene = Scene::new(vec![Layer::new(solid(RED))]);
    assert_eq!(line(&scene, 4), vec![RED; 4]);
  }

  #[test]
  fn gradient_on_a_line_matches_color_sequence() {
    use crate::prelude::*;

    let scene = Scene::new(vec![Layer::new(red_to_blue())]);
    for count in [1, 2, 5, 16, 33] {
      let expected = ColorSequence::fade(RED, BLUE).generate(count);
      let actual = line(&scene, count);
      for (i, (a, e)) in actual.iter().zip(&expected).enumerate() {
        let close = (0..4).all(|c| (a[c] as i16 - e[c] as i16).abs() <= 1);
        assert!(close, "{count} LEDs, LED {i}: {a:?} vs ColorSequence {e:?}");
      }
    }
  }

  #[test]
  fn shapes_on_a_grid() {
    let scene = circle(solid(RED));
    // a 100 x 50 DMD, one sample per pixel
    let lit = (0..50)
      .flat_map(|y| (0..100).map(move |x| (x, y)))
      .map(|(x, y)| sample(&scene, x as f32 + 0.5, y as f32 + 0.5, 0.5))
      .filter(|c| *c == RED)
      .count();
    // close to the area of the circle, less the partly covered edge
    let area = std::f32::consts::PI * 20.0 * 20.0;
    assert!((lit as f32 - area).abs() < 2.0 * std::f32::consts::PI * 20.0, "{lit} vs {area}");
    assert_eq!(sample(&scene, 50.0, 25.0, 0.5), RED);
    assert_eq!(sample(&scene, 5.0, 5.0, 0.5), DARK);
  }

  #[test]
  fn shapes_at_sparse_points() {
    let scene = circle(solid(RED));
    // inserts with a large footprint, one well inside, one well outside, one straddling the edge
    assert_eq!(sample(&scene, 50.0, 25.0, 3.0), RED);
    assert_eq!(sample(&scene, 90.0, 25.0, 3.0), DARK);
    let edge = sample(&scene, 70.0, 25.0, 3.0);
    assert!(edge[0] > DARK[0] && edge[0] < RED[0], "{edge:?}");
  }

  #[test]
  fn gradient_fills_its_shape() {
    let scene = circle(red_to_blue());
    assert_eq!(sample(&scene, 30.5, 25.0, 0.0), Rgba([252, 0, 3, 255]));
    assert_eq!(sample(&scene, 69.5, 25.0, 0.0), Rgba([3, 0, 252, 255]));
  }

  #[test]
  fn gradient_turns_with_its_shape() {
    let mut scene = circle(red_to_blue());
    let Some(shape) = &mut scene.layers[1].shape else { unreachable!() };
    shape.warp.insert(1, Warp::Rotate(90.0.into()));
    // the red end is now at the top, and the blue end at the bottom
    assert_eq!(sample(&scene, 50.0, 5.5, 0.0), Rgba([252, 0, 3, 255]));
    assert_eq!(sample(&scene, 50.0, 44.5, 0.0), Rgba([3, 0, 252, 255]));
  }

  #[test]
  fn shape_on_a_line_uses_led_indexes() {
    // a circle 3 LEDs across centered on the third LED lights LEDs 1 to 3
    let scene = Scene::new(vec![Layer::new(solid(RED)).shape(
      Shape::new(ShapeNode::Ellipse)
        .warp(Warp::Scale(Vec2::splat(3.0).into()))
        .warp(Warp::Translate(Vec2::new(2.0, 0.0).into())),
    )]);
    let transparent = Rgba([0, 0, 0, 0]);
    assert_eq!(line(&scene, 5), vec![transparent, RED, RED, RED, transparent]);
  }

  #[test]
  fn params_override_defaults() {
    use crate::scene::{Prop, Value};

    let mut scene = Scene::new(vec![Layer::new(Paint::new(PaintNode::Solid(Prop::Param(
      "color".into(),
    ))))]);
    scene.params.insert("color".into(), Value::Color(Color(RED)));
    assert_eq!(line(&scene, 1), vec![RED]);

    let params = Params::from([("color".into(), Value::Color(Color(BLUE)))]);
    let frame = Frame { time: 0.0, stage: Line::new(1).stage(), params: &params };
    assert_eq!(scene.sample(&Sample::new(Vec2::ZERO, 0.5), &frame), BLUE);
  }
}
