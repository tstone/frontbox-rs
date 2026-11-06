use std::borrow::Cow;

use frontbox_core::prelude::*;
use frontbox_gfx::layout::Line;
use frontbox_gfx::scene::{Bounds, Frame, Params, Sample, Scene, Value, Viewport};

use crate::*;

/// Plays a [`frontbox_gfx::scene::Scene`] on LEDs
///
/// How a scene lands on its LEDs depends on its viewport. On a `Sequence` viewport, a list of LEDs spreads along it in
/// order, however many they are:
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # use frontbox_led::prelude::*;
/// # use frontbox_gfx::scene::*;
/// let scene = file::load(r#"(
///   version: 5,
///   viewport: Sequence,
///   layers: [(paint: (base: Solid(Param("color"))))],
/// )"#).unwrap();
///
/// let mut program = LedScene::new(LedQ::every(), scene).unwrap();
/// program.set_param("color", Value::Color(Color(Rgba::red())));
/// ```
///
/// On an `Area` viewport, LEDs light up from where they physically are on a plane. The scene covers the plane, and its
/// numbers are in the plane's units:
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # use frontbox_led::prelude::*;
/// # use frontbox_gfx::scene::*;
/// static PLAYFIELD: ReferencePlane = ReferencePlane::new("Playfield").extent(Vec2::new(20.25, 45.0)).build();
///
/// let scene = file::load(r#"(
///   version: 5,
///   viewport: Area,
///   layers: [(paint: (base: Solid(Fixed(255, 0, 0, 255))))],
/// )"#).unwrap();
///
/// let program = LedScene::on_plane(&PLAYFIELD, LedQ::every(), scene).unwrap().at_z(2);
/// ```
#[derive(Clone)]
pub struct LedScene {
  targets: Targets,
  scene: Scene,
  params: Params,
  elapsed: Duration,
  take: u32,
  playing: bool,
  end_behavior: EndBehavior,
  undeclared: bool,
}

impl LedScene {
  /// A list of LEDs has no positions to place on an `Area` viewport, so a scene with one is refused
  pub fn new<T: Contextual<LedIdentifications> + Send + Sync + 'static>(
    targets: T,
    scene: Scene,
  ) -> Result<Self, NotASequence> {
    if scene.viewport != Viewport::Sequence {
      return Err(NotASequence);
    }
    Ok(Self::targeting(Targets::Sequence(Box::new(targets)), scene))
  }

  /// Plays on the LEDs matching `query` that have a location, projected straight onto `plane`. LEDs that land outside
  /// the plane's extent don't take part. A `Sequence` viewport has no area to project onto and a plane with no extent
  /// has no area to play on, so either is refused.
  pub fn on_plane(plane: &ReferencePlane, query: LedQ, scene: Scene) -> Result<Self, PlaneSceneError> {
    if scene.viewport != Viewport::Area {
      return Err(PlaneSceneError::NotAnArea);
    }
    if plane.extent.min_element() <= 0.0 {
      return Err(PlaneSceneError::NoExtent);
    }
    let targets = Targets::Plane {
      plane: plane.clone(),
      query,
      z_index: 0,
      placement: None,
    };
    Ok(Self::targeting(targets, scene))
  }

  fn targeting(targets: Targets, scene: Scene) -> Self {
    Self {
      targets,
      scene,
      params: Params::new(),
      elapsed: Duration::ZERO,
      take: rand::random(),
      playing: true,
      end_behavior: EndBehavior::Clear,
      undeclared: false,
    }
  }

  /// What to do when the scene is over. It clears unless told otherwise.
  pub fn end_behavior(mut self, end_behavior: EndBehavior) -> Self {
    self.end_behavior = end_behavior;
    self
  }

  /// The layer to declare the LEDs on
  pub fn at_z(mut self, z_index: i8) -> Self {
    self.targets = match self.targets {
      Targets::Sequence(ids) => Targets::Sequence(Box::new(ids.at_z(z_index))),
      Targets::Plane { plane, query, .. } => Targets::Plane {
        plane,
        query,
        z_index,
        placement: None,
      },
    };
    self
  }

  /// Sets a parameter, taking the place of the scene's default
  pub fn set_param(&mut self, name: impl Into<String>, value: Value) {
    self.params.insert(name.into(), value);
  }

  /// Move time forward and declare the LEDs' colors
  pub fn apply(&mut self, delta: Duration, ctx: &SystemContext) {
    if !self.playing {
      return;
    }
    if self.is_complete() && self.end_behavior == EndBehavior::Clear {
      if !self.undeclared {
        let placement = self.targets.place(ctx);
        ctx.undeclare_leds(&placement.ids);
        self.undeclared = true;
      }
      return;
    }
    self.elapsed += delta;

    let placement = self.targets.place(ctx);
    ctx.declare_led_colors(&placement.ids, |_| {
      let frame = Frame { time: self.elapsed.as_secs_f32(), take: self.take, params: &self.params, size: placement.size };
      placement.samples.iter().map(|s| self.scene.sample(s, &frame)).collect()
    });
  }

  /// Play from where the scene is, restarting it if it has already completed
  pub fn play(&mut self) {
    if self.is_complete() {
      self.reset();
    }
    self.playing = true;
  }

  /// Stop playing and clear the LEDs
  pub fn stop(&mut self, ctx: &SystemContext) {
    self.playing = false;
    let placement = self.targets.place(ctx);
    ctx.undeclare_leds(&placement.ids);
  }

  /// Back to the start of the scene
  pub fn reset(&mut self) {
    self.elapsed = Duration::ZERO;
    self.take = rand::random();
    self.undeclared = false;
  }

  pub fn is_playing(&self) -> bool {
    self.playing
  }

  /// Whether the scene has played through. A scene that's static or repeats forever never completes, and plays until
  /// it's stopped.
  pub fn is_complete(&self) -> bool {
    self.scene.is_complete(self.elapsed.as_secs_f32())
  }
}

/// A scene given to [`LedScene::new`] with a viewport other than `Sequence`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotASequence;

impl std::fmt::Display for NotASequence {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "a list of LEDs can only play a scene with a Sequence viewport")
  }
}

impl std::error::Error for NotASequence {}

/// Why [`LedScene::on_plane`] can't play a scene on a plane
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaneSceneError {
  /// The scene's viewport isn't an `Area`
  NotAnArea,
  /// The plane has no extent, which is what gives a scene its size
  NoExtent,
}

impl std::fmt::Display for PlaneSceneError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::NotAnArea => write!(f, "LEDs on a plane can only play a scene with an Area viewport"),
      Self::NoExtent => write!(f, "a scene can only play on a plane with an extent"),
    }
  }
}

impl std::error::Error for PlaneSceneError {}

/// The LEDs matching `query` that land within `plane`'s extent when projected straight onto it, each with where it
/// samples a scene. LEDs without a location don't take part.
pub fn place_on_plane<'a>(
  leds: &'a LedLookup,
  plane: &ReferencePlane,
  query: &'a LedQ,
  radius: f32,
) -> Vec<(&'a LED, Sample)> {
  let extent = Bounds::sized(plane.extent);
  leds
    .query_iter(query)
    .filter_map(|led| {
      let pos = plane.to_relative(led.location?);
      extent.contains(pos).then(|| (led, Sample::new(pos, radius)))
    })
    .collect()
}

/// The LEDs a scene plays on
#[derive(Clone)]
enum Targets {
  Sequence(Box<dyn Contextual<LedIdentifications> + Send + Sync>),
  Plane {
    plane: ReferencePlane,
    query: LedQ,
    z_index: i8,
    /// LEDs don't move, so they're only found once
    placement: Option<Placement>,
  },
}

impl Targets {
  fn place(&mut self, ctx: &SystemContext) -> Cow<'_, Placement> {
    match self {
      Self::Sequence(ids) => {
        let ids = ids.resolve(ctx);
        let samples = Line::new(ids.leds.len()).samples().collect();
        Cow::Owned(Placement { ids, samples, size: Vec2::ZERO })
      }
      Self::Plane { plane, query, z_index, placement } => {
        Cow::Borrowed(placement.get_or_insert_with(|| Placement::on_plane(plane, query, *z_index, ctx)))
      }
    }
  }
}

/// Which LEDs a scene lands on, and each one's sample of it, in the same order
#[derive(Clone)]
struct Placement {
  ids: LedIdentifications,
  samples: Vec<Sample>,
  /// The size of what they're on
  size: Vec2,
}

impl Placement {
  fn on_plane(plane: &ReferencePlane, query: &LedQ, z_index: i8, ctx: &SystemContext) -> Self {
    let radius = ctx.systems.get::<LedSystem>().map_or(DEFAULT_LED_RADIUS, |system| system.led_radius);

    let (leds, samples) = place_on_plane(&ctx.leds, plane, query, radius)
      .into_iter()
      .map(|(led, sample)| (led.address.clone(), sample))
      .unzip();

    Self { ids: LedIdentifications::new(leds, z_index), samples, size: plane.extent }
  }
}

#[cfg(test)]
mod tests {
  use frontbox_gfx::animation::{Curve, Cycle};
  use frontbox_gfx::scene::*;
  use frontbox_gfx::prelude::*;

  use super::*;

  fn three_leds() -> (TestContext, Vec<LedAddress>) {
    let mut context = TestContext::default();
    context.insert_system(LedSystem::new());
    let addresses: Vec<_> = (1..=3).map(|i| LedAddress::new(ExpAddress::default(), i)).collect();
    for (i, address) in addresses.iter().enumerate() {
      context.insert_led(LED {
        name: format!("led{}", i + 1),
        address: address.clone(),
        ..Default::default()
      });
    }
    (context, addresses)
  }

  fn colors(context: &TestContext, addresses: &[LedAddress]) -> Vec<Option<Rgba<u8>>> {
    let ctx = context.sys_ctx();
    let led_system = ctx.systems.expect::<LedSystem>();
    addresses
      .iter()
      .map(|a| led_system.declarations_for(a).first().map(|d| d.color))
      .collect()
  }

  fn scene(paint: PaintNode) -> Scene {
    Scene::new(Viewport::Sequence, vec![Layer::new(Paint::new(paint))])
  }

  #[test]
  fn gradient_runs_along_the_leds() {
    let (context, addresses) = three_leds();
    let mut program = LedScene::new(
      LedQ::names(vec!["led1", "led2", "led3"]),
      scene(PaintNode::Gradient {
        stops: vec![Stop::new(0.0, Color(Rgba::red())), Stop::new(1.0, Color(Rgba::blue()))],
        space: ColorSpace::Rgb,
      }),
    )
    .unwrap();

    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(
      colors(&context, &addresses),
      vec![Some(Rgba::red()), Some(Rgba([127, 0, 128, 255])), Some(Rgba::blue())]
    );
  }

  #[test]
  fn an_area_scene_is_refused() {
    let area = Scene::new(Viewport::Area, vec![]);
    assert!(matches!(LedScene::new(LedQ::every(), area), Err(NotASequence)));
  }

  #[test]
  fn time_moves_tracks() {
    let (context, addresses) = three_leds();
    let mut program = LedScene::new(
      LedQ::name("led1"),
      scene(PaintNode::Solid(Prop::Track(Track {
        keys: vec![
          Key::new(0.0, Color(Rgba([0, 0, 0, 255])), Curve::Linear),
          Key::new(1.0, Color(Rgba([200, 0, 0, 255])), Curve::Linear),
        ],
        cycle: Cycle::Once,
      })))
      .timing(Timing::new(1.0, Cycle::Once)),
    )
    .unwrap();

    program.apply(Duration::from_millis(500), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba([100, 0, 0, 255])));
  }

  fn timed(cycle: Cycle) -> Scene {
    scene(PaintNode::Solid(Color(Rgba::red()).into())).timing(Timing::new(1.0, cycle))
  }

  #[test]
  fn clears_the_leds_after_showing_the_last_moment() {
    let (context, addresses) = three_leds();
    let mut program = LedScene::new(LedQ::name("led1"), timed(Cycle::Once)).unwrap();

    program.apply(Duration::from_millis(500), &context.sys_ctx());
    assert!(!program.is_complete());

    program.apply(Duration::from_millis(500), &context.sys_ctx());
    assert!(program.is_complete());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));

    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], None);
    assert!(program.is_complete());
  }

  #[test]
  fn clearing_gives_the_leds_back_for_good() {
    let (context, addresses) = three_leds();
    let mut program = LedScene::new(LedQ::name("led1"), timed(Cycle::Once)).unwrap();
    program.apply(Duration::from_secs(1), &context.sys_ctx());
    program.apply(Duration::from_millis(16), &context.sys_ctx());

    // something else takes the LED, and a completed scene mustn't touch it
    let mut other = LedScene::new(LedQ::name("led1"), scene(PaintNode::Solid(Color(Rgba::blue()).into()))).unwrap();
    other.apply(Duration::from_millis(16), &context.sys_ctx());
    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::blue()));
  }

  #[test]
  fn can_hold_its_last_moment() {
    let (context, addresses) = three_leds();
    let mut program =
      LedScene::new(LedQ::name("led1"), timed(Cycle::Once)).unwrap().end_behavior(EndBehavior::Hold);

    program.apply(Duration::from_secs(1), &context.sys_ctx());
    program.apply(Duration::from_secs(1), &context.sys_ctx());
    assert!(program.is_complete());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));
  }

  #[test]
  fn each_play_picks_its_own_random_values() {
    let (context, addresses) = three_leds();
    let spread = Spread::Color { hue: (0.0, 360.0), brightness: (1.0, 1.0), alpha: (1.0, 1.0) };
    let color = Prop::Random(Random { picks: vec![Pick::new(0.0)], spread, seed: 0 });
    let scene = scene(PaintNode::Solid(color)).timing(Timing::new(1.0, Cycle::Once));
    let mut program = LedScene::new(LedQ::name("led1"), scene).unwrap();

    let mut shots = vec![];
    for _ in 0..8 {
      program.reset();
      program.apply(Duration::ZERO, &context.sys_ctx());
      shots.push(colors(&context, &addresses)[0]);
    }
    assert!(shots.iter().any(|shot| *shot != shots[0]), "{shots:?}");
  }

  #[test]
  fn play_restarts_a_completed_scene() {
    let (context, addresses) = three_leds();
    let mut program = LedScene::new(LedQ::name("led1"), timed(Cycle::Once)).unwrap();
    program.apply(Duration::from_secs(1), &context.sys_ctx());
    program.apply(Duration::from_millis(16), &context.sys_ctx());

    program.play();
    assert!(!program.is_complete());
    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));
  }

  #[test]
  fn play_continues_a_scene_that_was_stopped_partway() {
    let (context, _) = three_leds();
    let mut program = LedScene::new(LedQ::name("led1"), timed(Cycle::Once)).unwrap();
    program.apply(Duration::from_millis(600), &context.sys_ctx());
    program.stop(&context.sys_ctx());

    program.play();
    program.apply(Duration::from_millis(600), &context.sys_ctx());
    assert!(program.is_complete());
  }

  #[test]
  fn a_scene_that_repeats_forever_never_completes() {
    let (context, addresses) = three_leds();
    let mut program = LedScene::new(LedQ::name("led1"), timed(Cycle::Forever)).unwrap();
    program.apply(Duration::from_secs(100), &context.sys_ctx());
    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert!(!program.is_complete());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));
  }

  #[test]
  fn a_static_scene_never_completes() {
    let (context, addresses) = three_leds();
    let mut program = LedScene::new(LedQ::name("led1"), scene(PaintNode::Solid(Color(Rgba::red()).into()))).unwrap();
    program.apply(Duration::from_secs(100), &context.sys_ctx());
    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert!(!program.is_complete());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));
  }

  #[test]
  fn stop_clears_the_leds() {
    let (context, addresses) = three_leds();
    let mut program = LedScene::new(LedQ::name("led1"), scene(PaintNode::Solid(Color(Rgba::red()).into()))).unwrap();

    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));

    program.stop(&context.sys_ctx());
    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], None);
  }

  #[test]
  fn params_set_by_the_game() {
    let (context, addresses) = three_leds();
    let mut program = LedScene::new(
      LedQ::name("led1"),
      scene(PaintNode::Solid(Prop::Param("color".into()))),
    )
    .unwrap();
    program.set_param("color", Value::Color(Color(Rgba::green())));

    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::green()));
  }

  /// LEDs at these locations, named by position in the list
  fn placed_leds(system: LedSystem, locations: &[Option<Vec3>]) -> (TestContext, Vec<LedAddress>) {
    let mut context = TestContext::default();
    context.insert_system(system);
    let addresses: Vec<_> = (0..locations.len()).map(|i| LedAddress::new(ExpAddress::default(), i as u8)).collect();
    for (i, address) in addresses.iter().enumerate() {
      context.insert_led(LED {
        name: format!("led{i}"),
        address: address.clone(),
        location: locations[i],
        ..Default::default()
      });
    }
    (context, addresses)
  }

  fn plane() -> ReferencePlane {
    ReferencePlane::new("Plane").extent(Vec2::new(10.0, 10.0)).build()
  }

  fn at(x: f32, y: f32) -> Option<Vec3> {
    Some(Vec3::new(x, y, 0.0))
  }

  fn area(paint: PaintNode) -> Scene {
    Scene::new(Viewport::Area, vec![Layer::new(Paint::new(paint))])
  }

  fn red() -> Scene {
    area(PaintNode::Solid(Color(Rgba::red()).into()))
  }

  fn red_to_blue() -> PaintNode {
    PaintNode::Gradient {
      stops: vec![Stop::new(0.0, Color(Rgba::red())), Stop::new(1.0, Color(Rgba::blue()))],
      space: ColorSpace::Rgb,
    }
  }

  fn played(context: &TestContext, addresses: &[LedAddress], mut program: LedScene) -> Vec<Option<Rgba<u8>>> {
    program.apply(Duration::from_millis(16), &context.sys_ctx());
    colors(context, addresses)
  }

  #[test]
  fn a_sequence_scene_is_refused_on_a_plane() {
    let result = LedScene::on_plane(&plane(), LedQ::every(), scene(PaintNode::Solid(Color(Rgba::red()).into())));
    assert!(matches!(result, Err(PlaneSceneError::NotAnArea)));
  }

  #[test]
  fn a_plane_with_no_extent_is_refused() {
    let flat = ReferencePlane::new("Flat").build();
    let result = LedScene::on_plane(&flat, LedQ::every(), red());
    assert!(matches!(result, Err(PlaneSceneError::NoExtent)));
  }

  #[test]
  fn a_scene_takes_its_size_from_the_plane() {
    // the same scene on a plane twice as wide: halfway across its gradient is further along
    let wide = ReferencePlane::new("Wide").extent(Vec2::new(20.0, 10.0)).build();
    let (context, addresses) = placed_leds(LedSystem::new(), &[at(20.0, 5.0)]);
    let program = LedScene::on_plane(&wide, LedQ::every(), area(red_to_blue())).unwrap();

    assert_eq!(played(&context, &addresses, program), vec![Some(Rgba::blue())]);
  }

  #[test]
  fn colors_come_from_where_leds_are_not_their_order() {
    // out of order and unevenly spread: along a list these would be red, purple, blue
    let (context, addresses) = placed_leds(LedSystem::new(), &[at(10.0, 5.0), at(0.0, 5.0), at(5.0, 5.0)]);
    let program = LedScene::on_plane(&plane(), LedQ::every(), area(red_to_blue())).unwrap();

    assert_eq!(
      played(&context, &addresses, program),
      vec![Some(Rgba::blue()), Some(Rgba::red()), Some(Rgba([127, 0, 128, 255]))]
    );
  }

  #[test]
  fn only_leds_matching_the_query_take_part() {
    let (context, addresses) = placed_leds(LedSystem::new(), &[at(1.0, 1.0), at(2.0, 2.0)]);
    let program = LedScene::on_plane(&plane(), LedQ::name("led1"), red()).unwrap();

    assert_eq!(played(&context, &addresses, program), vec![None, Some(Rgba::red())]);
  }

  #[test]
  fn leds_without_a_location_dont_take_part() {
    let (context, addresses) = placed_leds(LedSystem::new(), &[None, at(2.0, 2.0)]);
    let program = LedScene::on_plane(&plane(), LedQ::every(), red()).unwrap();

    assert_eq!(played(&context, &addresses, program), vec![None, Some(Rgba::red())]);
  }

  #[test]
  fn leds_outside_the_viewport_dont_take_part() {
    let (context, addresses) = placed_leds(LedSystem::new(), &[at(2.0, 2.0), at(11.0, 2.0)]);
    let program = LedScene::on_plane(&plane(), LedQ::every(), red()).unwrap();

    assert_eq!(played(&context, &addresses, program), vec![Some(Rgba::red()), None]);
  }

  #[test]
  fn positions_are_relative_to_the_plane() {
    // the plane starts at x = 100, so x = 110 is its far edge and x = 5 is off it
    let offset = ReferencePlane::new("Offset").origin(Vec3::new(100.0, 0.0, 0.0)).extent(Vec2::new(10.0, 10.0)).build();
    let (context, addresses) = placed_leds(LedSystem::new(), &[at(110.0, 5.0), at(5.0, 5.0)]);
    let program = LedScene::on_plane(&offset, LedQ::every(), area(red_to_blue())).unwrap();

    assert_eq!(played(&context, &addresses, program), vec![Some(Rgba::blue()), None]);
  }

  #[test]
  fn leds_off_the_surface_project_straight_onto_it() {
    let (context, addresses) = placed_leds(LedSystem::new(), &[Some(Vec3::new(0.0, 5.0, 1.5))]);
    let program = LedScene::on_plane(&plane(), LedQ::every(), area(red_to_blue())).unwrap();

    assert_eq!(played(&context, &addresses, program), vec![Some(Rgba::red())]);
  }

  #[test]
  fn led_radius_softens_shape_edges() {
    // how much of a circle's edge shows on an LED just inside it
    let coverage = |system: LedSystem| {
      let (context, addresses) = placed_leds(system, &[at(5.0, 6.95)]);
      let circle = Layer::new(Paint::new(PaintNode::Solid(Color(Rgba::red()).into()))).shape(
        Shape::new(ShapeNode::Ellipse)
          .warp(Warp::Scale(Vec2::splat(4.0).into()))
          .warp(Warp::Translate(Vec2::new(5.0, 5.0).into())),
      );
      let scene = Scene::new(Viewport::Area, vec![circle]);
      let program = LedScene::on_plane(&plane(), LedQ::every(), scene).unwrap();
      played(&context, &addresses, program)[0].unwrap()[3]
    };

    let default = coverage(LedSystem::new());
    let soft = coverage(LedSystem::new().led_radius(2.0));
    assert!(soft < default && soft > 127, "{soft} vs {default}");
  }

  #[test]
  fn stop_clears_leds_on_their_layer() {
    let (context, addresses) = placed_leds(LedSystem::new(), &[at(2.0, 2.0)]);
    let mut program = LedScene::on_plane(&plane(), LedQ::every(), red()).unwrap().at_z(3);

    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));

    program.stop(&context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], None);
  }

  #[test]
  fn each_z_is_its_own_layer() {
    let (context, addresses) = three_leds();
    let solid = |color: Rgba<u8>| scene(PaintNode::Solid(Color(color).into()));
    let mut under = LedScene::new(LedQ::name("led1"), solid(Rgba::red())).unwrap();
    let mut over = LedScene::new(LedQ::name("led1"), solid(Rgba::blue())).unwrap().at_z(1);
    let layers = || context.sys_ctx().systems.expect::<LedSystem>().declarations_for(&addresses[0]).len();

    under.apply(Duration::from_millis(16), &context.sys_ctx());
    over.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(layers(), 2);

    over.stop(&context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));
  }
}
