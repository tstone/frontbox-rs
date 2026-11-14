//! Drawing one frame of a layer on a 128x32 display, for each kind of layer.
//!
//! `cargo bench -p frontbox-gfx` for a release build, and `cargo bench -p frontbox-gfx --profile dev` for a debug one.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use frontbox_gfx::animation::{Curve, Cycle};
use frontbox_gfx::layout::Grid;
use frontbox_gfx::scene::*;
use glam::Vec2;
use image::{Rgba, RgbaImage};

const SIZE: Vec2 = Vec2::new(128.0, 32.0);
const RED: Color = Color(Rgba([255, 0, 0, 255]));
const BLUE: Color = Color(Rgba([0, 0, 255, 255]));
const WHITE: Color = Color(Rgba([255, 255, 255, 255]));

fn layers() -> Vec<(&'static str, Layer)> {
  vec![
    ("solid", Layer::new(Fill::new(FillNode::Solid(RED.into())))),
    ("gradient", Layer::new(gradient(GradientStyle::Linear).transform(Transform::Rotate(over_time(0.0, 360.0))))),
    (
      "moving rect",
      Layer::new(Fill::new(FillNode::Solid(BLUE.into()))).region(
        Shape::new(ShapeNode::Rect { width: 40.0.into(), height: 12.0.into() })
          .transform(Transform::Rotate(over_time(0.0, 90.0)))
          .transform(Transform::Translate(Prop::Keyframes(Keyframes::new(
            vec![Key::new(0.0, Vec2::new(20.0, 16.0), Curve::EaseInOut), Key::new(1.0, Vec2::new(108.0, 16.0), Curve::Linear)],
            Cycle::Once,
          )))),
      ),
    ),
    (
      "polygon",
      Layer::new(gradient(GradientStyle::Radial)).region(
        Shape::new(ShapeNode::Polygon {
          points: [(0.0, -14.0), (12.0, -4.0), (8.0, 12.0), (-8.0, 12.0), (-12.0, -4.0)].map(|(x, y)| Vec2::new(x, y).into()).to_vec(),
        })
        .transform(Transform::Translate(Vec2::new(64.0, 16.0).into())),
      ),
    ),
    (
      "sweep",
      Layer::new(Fill::new(FillNode::Solid(WHITE.into()))).region(
        Shape::new(ShapeNode::circle(4.0)).transform(Transform::Translate(Prop::Sweep(Sweep::new(
          Shape::new(ShapeNode::Bezier {
            points: vec![
              BezierPoint { point: Vec2::new(8.0, 24.0).into(), handle: Vec2::new(20.0, -20.0).into() },
              BezierPoint { point: Vec2::new(64.0, 8.0).into(), handle: Vec2::new(20.0, 0.0).into() },
              BezierPoint { point: Vec2::new(120.0, 24.0).into(), handle: Vec2::new(20.0, 20.0).into() },
            ],
          }),
          over_time(0.0, 1.0),
        )))),
      ),
    ),
    ("small circle", Layer::new(Fill::new(FillNode::Solid(RED.into()))).region(Shape::new(ShapeNode::circle(3.0)).transform(Transform::Translate(Vec2::new(10.0, 10.0).into())))),
    ("image", Layer::new(Fill::new(FillNode::image("checkers")))),
    (
      "masked",
      Layer::new(gradient(GradientStyle::Conical)).mask(
        Mask::new(Fill::new(FillNode::Solid(WHITE.into()))).region(Shape::new(ShapeNode::circle(14.0)).transform(Transform::Translate(Vec2::new(64.0, 16.0).into()))),
      ),
    ),
    (
      "adjusted",
      Layer::new(
        gradient(GradientStyle::Linear)
          .adjust(ColorAdjust::HueShift(over_time(0.0, 180.0)))
          .adjust(ColorAdjust::Brightness(over_time(1.0, 0.5))),
      ),
    ),
  ]
}

fn gradient(style: GradientStyle) -> Fill {
  Fill::new(FillNode::Gradient { stops: vec![Stop::new(0.0, RED), Stop::new(0.5, WHITE), Stop::new(1.0, BLUE)], style })
}

fn over_time(from: f32, to: f32) -> Prop<f32> {
  Prop::Keyframes(Keyframes::new(vec![Key::new(0.0, from, Curve::Linear), Key::new(1.0, to, Curve::Linear)], Cycle::Once))
}

fn images() -> Images {
  let mut images = Images::new();
  let checkers = RgbaImage::from_fn(64, 16, |x, y| if (x + y) % 2 == 0 { Rgba([255, 255, 0, 255]) } else { Rgba([0, 0, 0, 255]) });
  images.insert("checkers", Picture::new(checkers));
  images
}

fn draw(c: &mut Criterion) {
  let params = Params::new();
  let images = images();
  let frame = LayerFrame {
    time: 0.4,
    timing: Some(Timing::new(1.0, Cycle::Forever)),
    playback_seed: 0,
    params: &params,
    defaults: no_params(),
    images: &images,
    size: SIZE,
    surface: Surface::Area,
    visits: None,
  };
  let grid = Grid::new(128, 32, SIZE);
  let mut pixels = vec![Rgba([0, 0, 0, 0]); grid.width * grid.height];

  let layers = layers();
  let mut group = c.benchmark_group("128x32");
  for (name, layer) in &layers {
    group.bench_function(*name, |b| {
      b.iter(|| {
        pixels.fill(Rgba([0, 0, 0, 0]));
        layer.sample_grid(&grid, black_box(&frame), &mut pixels);
      })
    });
  }
  group.bench_function("all of them", |b| {
    b.iter(|| {
      pixels.fill(Rgba([0, 0, 0, 0]));
      for (_, layer) in &layers {
        layer.sample_grid(&grid, black_box(&frame), &mut pixels);
      }
    })
  });
  group.finish();
}

criterion_group! {
  name = benches;
  config = Criterion::default().sample_size(10);
  targets = draw
}
criterion_main!(benches);
