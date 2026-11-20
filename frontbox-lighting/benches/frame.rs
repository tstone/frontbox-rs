//! One light frame, as the run loop does it each tick: the `SceneSystem` ticks, then `LayeredLightManager` ticks and
//! renders (draws the playbacks, resolves each light, lowers to LED addresses, and sends what changed). Run at a few
//! machine sizes, for each kind of work a frame can have.
//!
//! No machine is running, so sends stop at `ctx.lights.set`: the time includes putting colors into channel order but
//! not the serial port.
//!
//! `cargo bench -p frontbox-lighting` for a release build, and `cargo bench -p frontbox-lighting --profile dev` for a
//! debug one.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use frontbox_core::prelude::*;
use frontbox_gfx::animation::{Curve, Cycle};
use frontbox_gfx::prelude::*;
use frontbox_gfx::scene::*;
use frontbox_lighting::*;
use frontbox_scene::*;

/// Light counts: a typical machine, and one with a few long strips of cabinet lighting on top
const SIZES: [usize; 3] = [300, 600, 1200];
const FRAME: Duration = Duration::from_millis(16);
/// LEDs per breakout, below the 256 an LED index can address
const LEDS_PER_BREAKOUT: usize = 200;
const COLUMNS: usize = 20;
const PLAYFIELD_SIZE: Vec2 = Vec2::new(50.0, 100.0);

static PLAYFIELD: ReferencePlane = ReferencePlane::new("Playfield").extent(PLAYFIELD_SIZE).build();

/// What a frame has to do, besides drawing nothing
fn cases() -> Vec<(&'static str, fn(&TestContext))> {
  vec![
    ("declared", declare_status),
    ("area sweep", |context| play(context, area_sweep())),
    ("sequence", |context| play(context, sequence())),
    ("graph wave", |context| play(context, graph_wave())),
    ("everything", |context| {
      declare_status(context);
      play(context, area_sweep());
      play(context, sequence());
      play(context, graph_wave());
    }),
  ]
}

/// `count` lights, "light0".., in a grid across the playfield, with light management and scenes running
fn machine(count: usize) -> TestContext {
  let mut context = TestContext::default();
  context.insert_system(LayeredLightManager::new());
  context.insert_system(SceneSystem::new());
  context.add_plane(&PLAYFIELD);
  let rows = count.div_ceil(COLUMNS);
  for i in 0..count {
    let exp = ExpAddress::new(0, Some((i / LEDS_PER_BREAKOUT) as u8));
    let (column, row) = ((i % COLUMNS) as f32, (i / COLUMNS) as f32);
    let location = Vec3::new(
      (column + 0.5) * PLAYFIELD_SIZE.x / COLUMNS as f32,
      (row + 0.5) * PLAYFIELD_SIZE.y / rows as f32,
      0.0,
    );
    context.insert_light(Light {
      name: format!("light{i}"),
      address: LightAddress::rgb(LedAddress::new(exp, 0, (i % LEDS_PER_BREAKOUT) as u8)),
      location: Some(location),
      ..Default::default()
    });
  }
  context.sys_ctx().systems.with::<LayeredLightManager, _>(|lights| lights.on_spawn(&context.sys_ctx()));
  context
}

/// Ticks scenes and lights, and renders the lights
fn frame(context: &TestContext) {
  let ctx = context.sys_ctx();
  ctx.systems.with::<SceneSystem, _>(|scenes| scenes.on_tick(FRAME, &ctx));
  ctx.systems.with::<LayeredLightManager, _>(|lights| {
    lights.on_tick(FRAME, &ctx);
    lights.on_render(black_box(&ctx));
  });
}

/// Three systems declaring on every light, as status lights do: a color, a see-through one over it, and a flashing one
/// on every other light
fn declare_status(context: &TestContext) {
  let ctx = context.sys_ctx();
  let all: Vec<_> = ctx.lights.values().map(|light| light.address.clone()).collect();
  let colored = |color: Rgba<u8>, lights: &[LightAddress]| lights.iter().map(|light| (light.clone(), color)).collect::<Vec<_>>();
  let every_other: Vec<_> = all.iter().step_by(2).cloned().collect();
  ctx.systems.with::<LayeredLightManager, _>(|lights| {
    lights.declare(1, 0, &colored(Rgba::blue(), &all), LightMode::Solid);
    lights.declare(2, 1, &colored(Rgba([255, 0, 0, 128]), &all), LightMode::Solid);
    lights.declare(3, 2, &colored(Rgba::white(), &every_other), LightMode::Flashing);
  });
}

fn play(context: &TestContext, scene: Scene) {
  context.sys_ctx().systems.with::<SceneSystem, _>(|scenes| scenes.start(0, &PlayableScene::new(scene)));
}

/// A gradient turning across the playfield, changing every light each frame
fn area_sweep() -> Scene {
  let fill = gradient().transform(Transform::Rotate(over_time(0.0, 360.0)));
  Scene::new(vec![LightLayer::area(LightQ::every(), "Playfield", Layer::new(fill)).into()])
    .timing(Timing::new(1.0, Cycle::Forever))
}

/// A gradient along every light, its hue moving
fn sequence() -> Scene {
  let fill = gradient().adjust(ColorAdjust::HueShift(over_time(0.0, 360.0)));
  let layer = LightLayer { z: 1, ..LightLayer::sequence(LightQ::every(), Layer::new(fill)) };
  Scene::new(vec![layer.into()]).timing(Timing::new(1.0, Cycle::Forever))
}

/// Red spreading out from a corner over the lights' graph, fading behind
fn graph_wave() -> Scene {
  let shown = Prop::Visit(VisitResponse { smooth_out: Some(1.0), ..VisitResponse::new("wave", 0.0, 1.0) });
  let wave = Traversal::new("wave", NodeRef::name("light0"), TraversalAlgorithm::BreadthFirst { wander: 0.0.into() }, 20.0);
  let fill = Fill::new(FillNode::Solid(Color(Rgba::red()).into())).adjust(ColorAdjust::Opacity(shown));
  let layer = LightLayer {
    lights: Some(LightQ::every()),
    z: 2,
    surface: LightSurface::Area("Playfield".into()),
    layer: Layer::new(fill).graph(GraphMask::new(vec![wave])),
  };
  Scene::new(vec![layer.into()]).timing(Timing::new(5.0, Cycle::Forever))
}

fn gradient() -> Fill {
  Fill::new(FillNode::Gradient {
    stops: vec![
      Stop::new(0.0, Color(Rgba::red())),
      Stop::new(0.5, Color(Rgba::white())),
      Stop::new(1.0, Color(Rgba::blue())),
    ],
    style: GradientStyle::Linear,
  })
}

fn over_time(from: f32, to: f32) -> Prop<f32> {
  Prop::Keyframes(Keyframes::new(vec![Key::new(0.0, from, Curve::Linear), Key::new(1.0, to, Curve::Linear)], Cycle::Once))
}

fn frames(c: &mut Criterion) {
  let mut group = c.benchmark_group("frame");
  for (name, setup) in cases() {
    for count in SIZES {
      let context = machine(count);
      setup(&context);
      // the first frame places each playback's lights, which later frames reuse
      frame(&context);
      group.bench_with_input(BenchmarkId::new(name, count), &context, |b, context| b.iter(|| frame(context)));
    }
  }
  group.finish();
}

criterion_group! {
  name = benches;
  config = Criterion::default().sample_size(20);
  targets = frames
}
criterion_main!(benches);
