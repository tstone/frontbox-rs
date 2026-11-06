use glam::Vec2;
use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::color::RgbaColor;
use crate::scene::{Env, Frame, Layer, Params, Sample, Viewport, file};

/// A graphic and how it moves over time: layers seen through a viewport, and the parameters the game can change. Ready to be
/// sampled by any device, and written to a file with [`file`].
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
  pub version: u32,
  /// Parameter names and their defaults
  #[serde(default, skip_serializing_if = "Params::is_empty")]
  #[cfg_attr(feature = "ts", ts(as = "Option<Params>", optional))]
  pub params: Params,
  pub viewport: Viewport,
  /// Painted bottom to top, alpha over
  #[serde(default)]
  pub layers: Vec<Layer>,
}

impl Scene {
  /// A scene of the current format version
  pub fn new(viewport: Viewport, layers: Vec<Layer>) -> Self {
    Self { version: file::VERSION, params: Params::new(), viewport, layers }
  }

  /// The color at a sample. Transparent outside the viewport.
  pub fn sample(&self, at: &Sample, frame: &Frame) -> Rgba<u8> {
    let env = self.env(frame);
    let viewport = self.viewport.bounds(frame.size);
    if !viewport.contains(at.pos) {
      return Rgba([0, 0, 0, 0]);
    }
    self
      .layers
      .iter()
      .fold(Rgba([0, 0, 0, 0]), |below, layer| layer.sample(at, viewport, &env).composite_over(below))
  }

  /// Where each layer is drawn, as a closed loop of points in scene units. A layer with no shape outlines the viewport,
  /// and one that draws nothing has no points.
  pub fn outlines(&self, frame: &Frame) -> Vec<Vec<Vec2>> {
    let env = self.env(frame);
    let viewport = self.viewport.bounds(frame.size);
    self.layers.iter().map(|layer| layer.outline(viewport, &env)).collect()
  }

  fn env<'a>(&'a self, frame: &Frame<'a>) -> Env<'a> {
    Env { time: frame.time, params: frame.params, defaults: &self.params }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::layout::{Grid, Line};
  use crate::scene::{Color, ColorSpace, Paint, PaintNode, Shape, ShapeNode, Stop, Warp};

  const SIZE: Vec2 = Vec2::new(100.0, 50.0);
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
    let frame = Frame { time: 0.0, params: &params, size: SIZE };
    scene.sample(&Sample::new(Vec2::new(x, y), radius), &frame)
  }

  fn line(scene: &Scene, count: usize) -> Vec<Rgba<u8>> {
    let params = Params::new();
    let frame = Frame { time: 0.0, params: &params, size: SIZE };
    Line::new(count).samples().map(|s| scene.sample(&s, &frame)).collect()
  }

  /// A circle of diameter 40 at (50, 25) in a 100 x 50 viewport, over a dark background
  fn circle(paint: Paint) -> Scene {
    Scene::new(Viewport::Area, vec![
      Layer::new(solid(DARK)),
      Layer::new(paint).shape(
        Shape::new(ShapeNode::Ellipse)
          .warp(Warp::Scale(Vec2::splat(40.0).into()))
          .warp(Warp::Translate(Vec2::new(50.0, 25.0).into())),
      ),
    ])
  }

  fn outlines(layer: Layer) -> Vec<Vec2> {
    let params = Params::new();
    let frame = Frame { time: 0.0, params: &params, size: SIZE };
    Scene::new(Viewport::Area, vec![layer]).outlines(&frame).remove(0)
  }

  #[test]
  fn a_device_decides_the_size_of_an_area() {
    let scene = Scene::new(Viewport::Area, vec![Layer::new(solid(RED))]);
    let params = Params::new();
    let at = Sample::new(Vec2::new(150.0, 25.0), 0.0);
    let color = |size: Vec2| scene.sample(&at, &Frame { time: 0.0, params: &params, size });
    assert_eq!(color(Vec2::new(200.0, 50.0)), RED);
    assert_eq!(color(SIZE), Rgba([0, 0, 0, 0]));
  }

  #[test]
  fn a_gradient_with_no_shape_stretches_to_the_device() {
    let scene = Scene::new(Viewport::Area, vec![Layer::new(red_to_blue())]);
    let params = Params::new();
    let far_edge = |width: f32| {
      let frame = Frame { time: 0.0, params: &params, size: Vec2::new(width, 50.0) };
      scene.sample(&Sample::new(Vec2::new(width, 25.0), 0.0), &frame)
    };
    assert_eq!(far_edge(100.0), BLUE);
    assert_eq!(far_edge(300.0), BLUE);
  }

  #[test]
  fn a_layer_with_no_shape_outlines_the_viewport() {
    let corners = [(0.0, 0.0), (100.0, 0.0), (100.0, 50.0), (0.0, 50.0)].map(|(x, y)| Vec2::new(x, y));
    assert_eq!(outlines(Layer::new(solid(RED))), corners);
  }

  #[test]
  fn an_outline_follows_a_shapes_warps_in_order() {
    let square = ShapeNode::Polygon {
      points: [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| Vec2::new(x, y).into()).to_vec(),
    };
    let shape = Shape::new(square)
      .warp(Warp::Scale(Vec2::new(2.0, 4.0).into()))
      .warp(Warp::Translate(Vec2::new(10.0, 20.0).into()));
    let corners = [(10.0, 20.0), (12.0, 20.0), (12.0, 24.0), (10.0, 24.0)].map(|(x, y)| Vec2::new(x, y));
    assert_eq!(outlines(Layer::new(solid(RED)).shape(shape)), corners);
  }

  #[test]
  fn an_ellipse_outline_runs_around_its_edge() {
    let shape = Shape::new(ShapeNode::Ellipse)
      .warp(Warp::Scale(Vec2::new(40.0, 20.0).into()))
      .warp(Warp::Translate(Vec2::new(50.0, 25.0).into()));
    let outline = outlines(Layer::new(solid(RED)).shape(shape));
    assert!(outline.len() > 16);
    for point in outline {
      let offset = (point - Vec2::new(50.0, 25.0)) / Vec2::new(20.0, 10.0);
      assert!((offset.length() - 1.0).abs() < 1e-4, "{point}");
    }
  }

  #[test]
  fn a_shape_that_draws_nothing_has_no_outline() {
    let collapsed = Shape::new(ShapeNode::Ellipse).warp(Warp::Scale(Vec2::ZERO.into()));
    assert!(outlines(Layer::new(solid(RED)).shape(collapsed)).is_empty());
    let line = Shape::new(ShapeNode::Polygon { points: vec![Vec2::ZERO.into(), Vec2::ONE.into()] });
    assert!(outlines(Layer::new(solid(RED)).shape(line)).is_empty());
  }

  #[test]
  fn solid_on_a_line() {
    let scene = Scene::new(Viewport::Sequence, vec![Layer::new(solid(RED))]);
    assert_eq!(line(&scene, 4), vec![RED; 4]);
  }

  #[test]
  fn gradient_on_a_line_matches_color_sequence() {
    use crate::prelude::*;

    for count in [1, 2, 5, 16, 33] {
      let scene = Scene::new(Viewport::Sequence, vec![Layer::new(red_to_blue())]);
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
  fn shape_on_a_line_is_measured_along_it() {
    // on 5 LEDs, a circle most of the way across centered on the middle lights all but the end LEDs
    let scene = Scene::new(Viewport::Sequence, vec![Layer::new(solid(RED)).shape(
      Shape::new(ShapeNode::Ellipse)
        .warp(Warp::Scale(Vec2::splat(0.75).into()))
        .warp(Warp::Translate(Vec2::new(0.5, 0.0).into())),
    )]);
    let transparent = Rgba([0, 0, 0, 0]);
    assert_eq!(line(&scene, 5), vec![transparent, RED, RED, RED, transparent]);
  }

  #[test]
  fn a_sequence_plays_on_any_number_of_leds() {
    let scene = Scene::new(Viewport::Sequence, vec![Layer::new(red_to_blue())]);
    for count in [2, 6, 8, 60] {
      let colors = line(&scene, count);
      assert_eq!(colors[0], RED, "{count} LEDs");
      assert_eq!(colors[count - 1], BLUE, "{count} LEDs");
    }
  }

  #[test]
  fn a_shape_covers_the_same_part_of_any_strip() {
    let scene = Scene::new(Viewport::Sequence, vec![Layer::new(solid(RED)).shape(
      Shape::new(ShapeNode::Ellipse)
        .warp(Warp::Scale(Vec2::splat(0.5).into()))
        .warp(Warp::Translate(Vec2::new(0.5, 0.0).into())),
    )]);
    let lit = |count| line(&scene, count).iter().filter(|c| **c == RED).count();
    // the middle half, less the LEDs its edges pass through
    assert_eq!(lit(9), 3);
    assert_eq!(lit(101), 49);
  }

  #[test]
  fn nothing_is_seen_outside_the_viewport() {
    let scene = circle(solid(RED));
    let transparent = Rgba([0, 0, 0, 0]);
    assert_eq!(sample(&scene, -1.0, 25.0, 0.5), transparent);
    assert_eq!(sample(&scene, 101.0, 25.0, 0.5), transparent);
    assert_eq!(sample(&scene, 50.0, 51.0, 0.5), transparent);
  }

  #[test]
  fn a_shape_larger_than_the_viewport_can_move_under_it() {
    // a 300 wide gradient, moved so its middle is in view
    let scene = Scene::new(Viewport::Area, vec![Layer::new(red_to_blue()).shape(
      Shape::new(ShapeNode::Polygon {
        points: [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| Vec2::new(x, y).into()).to_vec(),
      })
      .warp(Warp::Scale(Vec2::new(300.0, 50.0).into()))
      .warp(Warp::Translate(Vec2::new(-100.0, 0.0).into())),
    )]);
    assert_eq!(sample(&scene, 50.0, 25.0, 0.0), Rgba([127, 0, 128, 255]));
  }

  #[test]
  fn a_grid_of_another_shape_does_not_squash_the_scene() {
    // 100 x 50 on a 128 x 32 grid: as wide as it is tall, with bars at the sides
    let scene = circle(solid(RED));
    let params = Params::new();
    let frame = Frame { time: 0.0, params: &params, size: SIZE };
    let pixels: Vec<_> =
      Grid::new(128, 32, Vec2::new(100.0, 50.0)).samples().map(|s| scene.sample(&s, &frame)).collect();
    let row = |y: usize| pixels[y * 128..(y + 1) * 128].iter().filter(|c| **c == RED).count();
    let column = |x: usize| (0..32).filter(|y| pixels[y * 128 + x] == RED).count();
    assert!(row(16).abs_diff(column(64)) <= 2, "{} across, {} down", row(16), column(64));
    assert_eq!(pixels[0], Rgba([0, 0, 0, 0]));
  }

  #[test]
  fn params_override_defaults() {
    use crate::scene::{Prop, Value};

    let mut scene = Scene::new(
      Viewport::Sequence,
      vec![Layer::new(Paint::new(PaintNode::Solid(Prop::Param("color".into()))))],
    );
    scene.params.insert("color".into(), Value::Color(Color(RED)));
    assert_eq!(line(&scene, 1), vec![RED]);

    let params = Params::from([("color".into(), Value::Color(Color(BLUE)))]);
    let frame = Frame { time: 0.0, params: &params, size: SIZE };
    assert_eq!(scene.sample(&Sample::new(Vec2::ZERO, 0.5), &frame), BLUE);
  }
}
