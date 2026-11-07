use std::fmt;

use frontbox_core::prelude::*;
use frontbox_gfx::layout::Line;
use frontbox_gfx::scene::{Frame, Images, Params, Sample, Value, Viewport};

use crate::*;

/// Plays an [`LedScene`]: the clock, parameters, and everything else about a scene that's only true while it's playing,
/// none of which is written to a file
///
/// A scene plays on the LEDs it says. Parameters are how code changes the rest of it:
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # use frontbox_led::prelude::*;
/// let scene = LedScene::load(r#"Led(
///   version: 1,
///   viewport: Sequence,
///   targets: Single(leds: Name("lane")),
///   composition: (layers: [(fill: (base: Solid(Param("color"))))]),
/// )"#).unwrap();
///
/// let program = LedPlayback::new(scene).unwrap().param("color", Rgba::red());
/// ```
///
/// A scene with a single target can be moved to other LEDs, for when which ones is only known in the game:
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # use frontbox_led::prelude::*;
/// # let scene = LedScene::load(r#"Led(
/// #   version: 1,
/// #   viewport: Sequence,
/// #   targets: Single(leds: Name("lane")),
/// #   composition: (layers: [(fill: (base: Solid(Fixed(255, 0, 0, 255))))]),
/// # )"#).unwrap();
/// let program = LedPlayback::with_leds(scene, LedQ::name("other_lane")).unwrap();
/// ```
#[derive(Clone)]
pub struct LedPlayback {
  scene: LedScene,
  /// Where each target lands, found the first time it plays as LEDs don't move
  placements: Option<Vec<Placement>>,
  params: Params,
  images: Images,
  elapsed: Duration,
  /// How much of the scene plays in a second
  speed: f32,
  take: u32,
  playing: bool,
  end_behavior: EndBehavior,
  undeclared: bool,
}

impl LedPlayback {
  /// A scene with no targets has nothing to play on, so it's refused
  pub fn new(scene: LedScene) -> Result<Self, PlaybackError> {
    if matches!(scene.targets, LedTargets::None) {
      return Err(PlaybackError::NoTargets);
    }
    Ok(Self {
      scene,
      placements: None,
      params: Params::new(),
      images: Images::new(),
      elapsed: Duration::ZERO,
      speed: 1.0,
      take: rand::random(),
      playing: true,
      end_behavior: EndBehavior::Clear,
      undeclared: false,
    })
  }

  /// Plays a scene's single target on other LEDs. Which layer it declares on, and the plane of an `Area`, stay as the
  /// scene says. A scene with no target to move, or several, is refused.
  pub fn with_leds(mut scene: LedScene, leds: LedQ) -> Result<Self, PlaybackError> {
    match &mut scene.targets {
      LedTargets::None => return Err(PlaybackError::NoTargets),
      LedTargets::Multiple(_) => return Err(PlaybackError::MultipleTargets),
      LedTargets::Single(target) => target.leds = leds,
    }
    Self::new(scene)
  }

  /// The images the scene draws, from [`Content::images_for`](frontbox_gfx::scene::Content::images_for). That reads
  /// files, so do it ahead of time rather than in a tick.
  pub fn images(mut self, images: Images) -> Self {
    self.images = images;
    self
  }

  /// What to do when the scene is over. It clears unless told otherwise.
  pub fn end_behavior(mut self, end_behavior: EndBehavior) -> Self {
    self.end_behavior = end_behavior;
    self
  }

  /// A scene starts out playing. Use this to chain it to prevent that, and it waits for [`LedPlayback::play`].
  pub fn stopped(mut self) -> Self {
    self.playing = false;
    self
  }

  /// Plays a cycle of the scene in `duration`, however long it was written to take, for when how long it lasts is only
  /// known once it's played
  pub fn over(mut self, duration: Duration) -> Self {
    if let Some(timing) = self.scene.composition.timing {
      self.speed = timing.duration / duration.as_secs_f32();
    }
    self
  }

  /// Sets a parameter, taking the place of the scene's default
  pub fn set_param(&mut self, name: impl Into<String>, value: impl Into<Value>) {
    self.params.insert(name.into(), value.into());
  }

  /// [`LedPlayback::set_param`] for chaining onto a new playback
  pub fn param(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
    self.set_param(name, value);
    self
  }

  /// Move time forward and declare the LEDs' colors
  pub fn apply(&mut self, delta: Duration, ctx: &SystemContext) {
    if !self.playing {
      return;
    }
    if self.is_complete() && self.end_behavior == EndBehavior::Clear {
      if !self.undeclared {
        self.undeclare(ctx);
        self.undeclared = true;
      }
      return;
    }
    self.elapsed += delta.mul_f32(self.speed);

    let scene = &self.scene;
    for placement in self.placements.get_or_insert_with(|| place(scene, ctx)) {
      let frame = Frame {
        time: self.elapsed.as_secs_f32(),
        take: self.take,
        params: &self.params,
        images: &self.images,
        size: placement.size,
        viewport: scene.viewport,
        target: placement.target.as_deref(),
      };
      let colors = placement.samples.iter().map(|s| scene.composition.sample(s, &frame));

      // a transparent declaration could still beat another system's on the same layer, so LEDs the scene isn't reaching
      // right now are given up instead
      let (shown, unreached): (Vec<_>, Vec<_>) =
        placement.ids.leds.iter().cloned().zip(colors).partition(|(_, color)| color[3] > 0);
      let (shown, colors): (Vec<_>, Vec<_>) = shown.into_iter().unzip();
      let unreached = unreached.into_iter().map(|(led, _)| led).collect();

      ctx.undeclare_leds(&LedIdentifications::new(unreached, placement.ids.z_index));
      ctx.declare_led_colors(&LedIdentifications::new(shown, placement.ids.z_index), |_| colors);
    }
  }

  fn undeclare(&self, ctx: &SystemContext) {
    for placement in self.placements.iter().flatten() {
      ctx.undeclare_leds(&placement.ids);
    }
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
    self.undeclare(ctx);
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
    self.scene.composition.is_complete(self.elapsed.as_secs_f32())
  }
}

/// Why a scene can't be played as it was asked to
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackError {
  /// The scene has no targets, so it isn't on any LEDs
  NoTargets,
  /// Code can only change the LEDs of a scene with a single target
  MultipleTargets,
}

impl fmt::Display for PlaybackError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NoTargets => write!(f, "the scene has no targets to play on"),
      Self::MultipleTargets => write!(f, "a scene with several targets can't be moved to other LEDs"),
    }
  }
}

impl std::error::Error for PlaybackError {}

/// The LEDs matching `query` that a scene on `viewport` lands on, each with where it samples the scene. This is what
/// [`LedPlayback`] does, for anything that wants to know what it would show without playing it.
///
/// On a `Sequence` they spread along it in order. An `Area` needs a `plane` to put them on, and without one reaches no
/// LEDs.
pub fn place_leds<'a>(
  viewport: Viewport,
  leds: &'a LedLookup,
  plane: Option<&ReferencePlane>,
  query: &'a LedQ,
  radius: f32,
) -> Vec<(&'a LED, Sample)> {
  match (viewport, plane) {
    (Viewport::Sequence, _) => {
      let matching: Vec<_> = leds.query_iter(query).collect();
      let line = Line::new(matching.len());
      matching.into_iter().zip(line.samples()).collect()
    }
    (Viewport::Area, Some(plane)) => place_on_plane(leds, plane, query, radius),
    (Viewport::Area, None) => Vec::new(),
  }
}

/// The LEDs matching `query`, each with where it samples a scene when projected straight onto `plane`, which may be past
/// the plane's extent. LEDs without a location don't take part.
fn place_on_plane<'a>(
  leds: &'a LedLookup,
  plane: &ReferencePlane,
  query: &'a LedQ,
  radius: f32,
) -> Vec<(&'a LED, Sample)> {
  leds
    .query_iter(query)
    .filter_map(|led| Some((led, Sample::new(plane.to_relative(led.location?), radius))))
    .collect()
}

/// Where each of a scene's targets lands
fn place(scene: &LedScene, ctx: &SystemContext) -> Vec<Placement> {
  scene.targets.iter().map(|(name, target)| Placement::of(name, target, scene.viewport, ctx)).collect()
}

/// Which LEDs a target lands on, and each one's sample of it, in the same order
#[derive(Clone)]
struct Placement {
  /// Which of the scene's layers it shows. `None` is all of them.
  target: Option<String>,
  ids: LedIdentifications,
  samples: Vec<Sample>,
  /// The size of what they're on
  size: Vec2,
}

impl Placement {
  fn of(name: Option<&str>, target: &LedTarget, viewport: Viewport, ctx: &SystemContext) -> Self {
    let plane = target.plane.as_deref().and_then(|plane| {
      let found = ctx.leds.plane(plane);
      if found.is_none() {
        log::error!("LedPlayback: there's no plane \"{plane}\" added to the app");
      }
      found
    });
    let (leds, samples) = place_leds(viewport, &ctx.leds, plane, &target.leds, ctx.led_radius)
      .into_iter()
      .map(|(led, sample)| (led.address.clone(), sample))
      .unzip();

    Self {
      target: name.map(str::to_string),
      ids: LedIdentifications::new(leds, target.z),
      samples,
      size: plane.map_or(Vec2::ZERO, |plane| plane.extent),
    }
  }
}

#[cfg(test)]
mod tests {
  use std::sync::LazyLock;

  use frontbox_gfx::animation::{Curve, Cycle};
  use frontbox_gfx::scene::*;
  use frontbox_gfx::prelude::*;
  use indexmap::IndexMap;

  use super::*;

  static PLANE: ReferencePlane = ReferencePlane::new("Plane").extent(Vec2::new(10.0, 10.0)).build();
  static WIDE: ReferencePlane = ReferencePlane::new("Wide").extent(Vec2::new(20.0, 10.0)).build();
  static OFFSET: LazyLock<ReferencePlane> =
    LazyLock::new(|| ReferencePlane::new("Offset").origin(Vec3::new(100.0, 0.0, 0.0)).extent(Vec2::new(10.0, 10.0)).build());

  /// Plays along some LEDs
  fn on(leds: LedQ, composition: GfxComposition) -> LedPlayback {
    let scene = LedScene::new(Viewport::Sequence, LedTargets::Single(LedTarget::new(leds)), composition);
    LedPlayback::new(scene).unwrap()
  }

  /// Plays across a plane
  fn across(plane: &str, leds: LedQ, composition: GfxComposition) -> LedPlayback {
    let target = LedTarget::new(leds).on_plane(plane);
    LedPlayback::new(LedScene::new(Viewport::Area, LedTargets::Single(target), composition)).unwrap()
  }

  fn on_plane(leds: LedQ, composition: GfxComposition) -> LedPlayback {
    across("Plane", leds, composition)
  }

  /// Plays on some named targets
  fn on_targets(viewport: Viewport, targets: Vec<(&str, LedTarget)>, composition: GfxComposition) -> LedPlayback {
    let targets = targets.into_iter().map(|(name, target)| (name.to_string(), target)).collect::<IndexMap<_, _>>();
    LedPlayback::new(LedScene::new(viewport, LedTargets::Multiple(targets), composition)).unwrap()
  }

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

  fn scene(fill: FillNode) -> GfxComposition {
    GfxComposition::new(vec![Layer::new(Fill::new(fill))])
  }

  #[test]
  fn an_image_runs_along_the_leds() {
    let (context, addresses) = three_leds();
    let art = image::RgbaImage::from_fn(8, 1, |x, _| if x < 4 { Rgba::red() } else { Rgba::blue() });
    let mut images = Images::new();
    images.insert("art.png", Picture::new(art));

    let mut program = on(LedQ::names(vec!["led1", "led2", "led3"]), scene(FillNode::image("art.png")))
      .images(images);

    program.apply(Duration::from_millis(16), &context.sys_ctx());
    let shown = colors(&context, &addresses);
    assert_eq!((shown[0], shown[2]), (Some(Rgba::red()), Some(Rgba::blue())));
  }

  #[test]
  fn gradient_runs_along_the_leds() {
    let (context, addresses) = three_leds();
    let mut program = on(LedQ::names(vec!["led1", "led2", "led3"]),
      scene(FillNode::Gradient {
        stops: vec![Stop::new(0.0, Color(Rgba::red())), Stop::new(1.0, Color(Rgba::blue()))],
        space: ColorSpace::Rgb,
        style: GradientStyle::Linear,
      }),
    );

    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(
      colors(&context, &addresses),
      vec![Some(Rgba::red()), Some(Rgba([127, 0, 128, 255])), Some(Rgba::blue())]
    );
  }

  #[test]
  fn a_stopped_scene_waits_for_play() {
    let (context, addresses) = three_leds();
    let mut program = on(LedQ::name("led1"), timed(Cycle::Once)).stopped();
    assert!(!program.is_playing());

    program.apply(Duration::from_secs(5), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], None);
    assert!(!program.is_complete());

    program.play();
    assert!(program.is_playing());
    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));
  }

  #[test]
  fn over_stretches_the_scene_to_a_duration() {
    let (context, _) = three_leds();
    let mut program = on(LedQ::name("led1"), timed(Cycle::Once)).over(Duration::from_secs(20));

    program.apply(Duration::from_secs(19), &context.sys_ctx());
    assert!(!program.is_complete());
    program.apply(Duration::from_secs(2), &context.sys_ctx());
    assert!(program.is_complete());
  }

  #[test]
  fn a_param_can_be_chained_on() {
    let (context, addresses) = three_leds();
    let program = on(LedQ::name("led1"), scene(FillNode::Solid(Prop::Param("color".into())))).param("color", Rgba::red());

    assert_eq!(played(&context, &addresses, program)[0], Some(Rgba::red()));
  }

  #[test]
  fn time_moves_tracks() {
    let (context, addresses) = three_leds();
    let mut program = on(LedQ::name("led1"), scene(FillNode::Solid(Prop::Track(Track {
        keys: vec![
          Key::new(0.0, Color(Rgba([0, 0, 0, 255])), Curve::Linear),
          Key::new(1.0, Color(Rgba([200, 0, 0, 255])), Curve::Linear),
        ],
        cycle: Cycle::Once,
      })))
      .timing(Timing::new(1.0, Cycle::Once)),
    );

    program.apply(Duration::from_millis(500), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba([100, 0, 0, 255])));
  }

  fn timed(cycle: Cycle) -> GfxComposition {
    scene(FillNode::Solid(Color(Rgba::red()).into())).timing(Timing::new(1.0, cycle))
  }

  #[test]
  fn clears_the_leds_after_showing_the_last_moment() {
    let (context, addresses) = three_leds();
    let mut program = on(LedQ::name("led1"), timed(Cycle::Once));

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
    let mut program = on(LedQ::name("led1"), timed(Cycle::Once));
    program.apply(Duration::from_secs(1), &context.sys_ctx());
    program.apply(Duration::from_millis(16), &context.sys_ctx());

    // something else takes the LED, and a completed scene mustn't touch it
    let mut other = on(LedQ::name("led1"), scene(FillNode::Solid(Color(Rgba::blue()).into())));
    other.apply(Duration::from_millis(16), &context.sys_ctx());
    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::blue()));
  }

  #[test]
  fn can_hold_its_last_moment() {
    let (context, addresses) = three_leds();
    let mut program =
      on(LedQ::name("led1"), timed(Cycle::Once)).end_behavior(EndBehavior::Hold);

    program.apply(Duration::from_secs(1), &context.sys_ctx());
    program.apply(Duration::from_secs(1), &context.sys_ctx());
    assert!(program.is_complete());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));
  }

  #[test]
  fn each_play_picks_its_own_random_values() {
    let (context, addresses) = three_leds();
    let spread = Spread::Color { hue: (0.0, 360.0), brightness: (1.0, 1.0), alpha: (1.0, 1.0) };
    let color = Prop::Random(Random { picks: vec![Pick::new(0.0)], spread, seed: 0, force_unique: true });
    let scene = scene(FillNode::Solid(color)).timing(Timing::new(1.0, Cycle::Once));
    let mut program = on(LedQ::name("led1"), scene);

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
    let mut program = on(LedQ::name("led1"), timed(Cycle::Once));
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
    let mut program = on(LedQ::name("led1"), timed(Cycle::Once));
    program.apply(Duration::from_millis(600), &context.sys_ctx());
    program.stop(&context.sys_ctx());

    program.play();
    program.apply(Duration::from_millis(600), &context.sys_ctx());
    assert!(program.is_complete());
  }

  #[test]
  fn a_scene_that_repeats_forever_never_completes() {
    let (context, addresses) = three_leds();
    let mut program = on(LedQ::name("led1"), timed(Cycle::Forever));
    program.apply(Duration::from_secs(100), &context.sys_ctx());
    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert!(!program.is_complete());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));
  }

  #[test]
  fn a_static_scene_never_completes() {
    let (context, addresses) = three_leds();
    let mut program = on(LedQ::name("led1"), scene(FillNode::Solid(Color(Rgba::red()).into())));
    program.apply(Duration::from_secs(100), &context.sys_ctx());
    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert!(!program.is_complete());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));
  }

  #[test]
  fn stop_clears_the_leds() {
    let (context, addresses) = three_leds();
    let mut program = on(LedQ::name("led1"), scene(FillNode::Solid(Color(Rgba::red()).into())));

    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));

    program.stop(&context.sys_ctx());
    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], None);
  }

  #[test]
  fn params_set_by_the_game() {
    let (context, addresses) = three_leds();
    let mut program = on(LedQ::name("led1"), scene(FillNode::Solid(Prop::Param("color".into()))),
    );
    program.set_param("color", Value::Color(Color(Rgba::green())));

    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::green()));
  }

  /// LEDs at these locations, named by position in the list
  fn placed_leds(locations: &[Option<Vec3>]) -> (TestContext, Vec<LedAddress>) {
    let mut context = TestContext::default();
    context.insert_system(LedSystem::new());
    let addresses: Vec<_> = (0..locations.len()).map(|i| LedAddress::new(ExpAddress::default(), i as u8)).collect();
    for (i, address) in addresses.iter().enumerate() {
      context.insert_led(LED {
        name: format!("led{i}"),
        address: address.clone(),
        location: locations[i],
        ..Default::default()
      });
    }
    context.add_plane(&PLANE);
    context.add_plane(&WIDE);
    context.add_plane(&OFFSET);
    (context, addresses)
  }

  fn at(x: f32, y: f32) -> Option<Vec3> {
    Some(Vec3::new(x, y, 0.0))
  }

  fn area(fill: FillNode) -> GfxComposition {
    GfxComposition::new(vec![Layer::new(Fill::new(fill))])
  }

  fn red() -> GfxComposition {
    area(FillNode::Solid(Color(Rgba::red()).into()))
  }

  fn red_to_blue() -> FillNode {
    FillNode::Gradient {
      stops: vec![Stop::new(0.0, Color(Rgba::red())), Stop::new(1.0, Color(Rgba::blue()))],
      space: ColorSpace::Rgb,
      style: GradientStyle::Linear,
    }
  }

  fn played(context: &TestContext, addresses: &[LedAddress], mut program: LedPlayback) -> Vec<Option<Rgba<u8>>> {
    program.apply(Duration::from_millis(16), &context.sys_ctx());
    colors(context, addresses)
  }

  #[test]
  fn a_scene_takes_its_size_from_the_plane() {
    // the same scene on a plane twice as wide: halfway across its gradient is further along
        let (context, addresses) = placed_leds(&[at(20.0, 5.0)]);
    let program = across("Wide", LedQ::every(), area(red_to_blue()));

    assert_eq!(played(&context, &addresses, program), vec![Some(Rgba::blue())]);
  }

  #[test]
  fn colors_come_from_where_leds_are_not_their_order() {
    // out of order and unevenly spread: along a list these would be red, purple, blue
    let (context, addresses) = placed_leds(&[at(10.0, 5.0), at(0.0, 5.0), at(5.0, 5.0)]);
    let program = on_plane(LedQ::every(), area(red_to_blue()));

    assert_eq!(
      played(&context, &addresses, program),
      vec![Some(Rgba::blue()), Some(Rgba::red()), Some(Rgba([127, 0, 128, 255]))]
    );
  }

  #[test]
  fn only_leds_matching_the_query_take_part() {
    let (context, addresses) = placed_leds(&[at(1.0, 1.0), at(2.0, 2.0)]);
    let program = on_plane(LedQ::name("led1"), red());

    assert_eq!(played(&context, &addresses, program), vec![None, Some(Rgba::red())]);
  }

  #[test]
  fn leds_without_a_location_dont_take_part() {
    let (context, addresses) = placed_leds(&[None, at(2.0, 2.0)]);
    let program = on_plane(LedQ::every(), red());

    assert_eq!(played(&context, &addresses, program), vec![None, Some(Rgba::red())]);
  }

  #[test]
  fn a_layer_with_no_shape_stops_at_the_edge_of_the_plane() {
    let (context, addresses) = placed_leds(&[at(2.0, 2.0), at(11.0, 2.0)]);
    let program = on_plane(LedQ::every(), red());

    assert_eq!(played(&context, &addresses, program), vec![Some(Rgba::red()), None]);
  }

  /// A red circle of radius 2 around this position
  fn red_circle(at: Prop<Vec2>) -> Shape {
    Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(Vec2::splat(4.0).into())).transform(Transform::Translate(at))
  }

  #[test]
  fn a_shape_reaches_leds_past_the_edge_of_the_plane() {
    let (context, addresses) = placed_leds(&[at(11.0, 5.0), at(20.0, 5.0)]);
    let fill = Fill::new(FillNode::Solid(Color(Rgba::red()).into()));
    let scene = GfxComposition::new(vec![Layer::new(fill).shape(red_circle(Vec2::new(11.0, 5.0).into()))]);
    let program = on_plane(LedQ::every(), scene);

    assert_eq!(played(&context, &addresses, program), vec![Some(Rgba::red()), None]);
  }

  #[test]
  fn an_led_a_shape_moves_off_is_given_up() {
    let (context, addresses) = placed_leds(&[at(5.0, 5.0)]);
    let fill = Fill::new(FillNode::Solid(Color(Rgba::red()).into()));
    let path = Track {
      keys: vec![Key::new(0.0, Vec2::new(5.0, 5.0), Curve::Linear), Key::new(1.0, Vec2::new(50.0, 5.0), Curve::Linear)],
      cycle: Cycle::Once,
    };
    let scene = GfxComposition::new(vec![Layer::new(fill).shape(red_circle(Prop::Track(path)))])
      .timing(Timing::new(1.0, Cycle::Once));
    let mut program = on_plane(LedQ::every(), scene).end_behavior(EndBehavior::Hold);

    program.apply(Duration::ZERO, &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));

    program.apply(Duration::from_secs(1), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], None);
  }

  #[test]
  fn positions_are_relative_to_the_plane() {
    // the plane starts at x = 100, so x = 110 is its far edge and x = 5 is off it
        let (context, addresses) = placed_leds(&[at(110.0, 5.0), at(5.0, 5.0)]);
    let program = across("Offset", LedQ::every(), area(red_to_blue()));

    assert_eq!(played(&context, &addresses, program), vec![Some(Rgba::blue()), None]);
  }

  #[test]
  fn leds_off_the_surface_project_straight_onto_it() {
    let (context, addresses) = placed_leds(&[Some(Vec3::new(0.0, 5.0, 1.5))]);
    let program = on_plane(LedQ::every(), area(red_to_blue()));

    assert_eq!(played(&context, &addresses, program), vec![Some(Rgba::red())]);
  }

  #[test]
  fn led_radius_softens_shape_edges() {
    // how much of a circle's edge shows on an LED just inside it
    let coverage = |radius: f32| {
      let (mut context, addresses) = placed_leds(&[at(5.0, 6.95)]);
      context.set_led_radius(radius);
      let circle = Layer::new(Fill::new(FillNode::Solid(Color(Rgba::red()).into()))).shape(
        Shape::new(ShapeNode::circle(0.5))
          .transform(Transform::Scale(Vec2::splat(4.0).into()))
          .transform(Transform::Translate(Vec2::new(5.0, 5.0).into())),
      );
      let scene = GfxComposition::new(vec![circle]);
      let program = on_plane(LedQ::every(), scene);
      played(&context, &addresses, program)[0].unwrap()[3]
    };

    let default = coverage(DEFAULT_LED_RADIUS);
    let soft = coverage(2.0);
    assert!(soft < default && soft > 127, "{soft} vs {default}");
  }

  #[test]
  fn place_leds_spreads_a_sequence_along_the_leds_in_order() {
    let (context, _) = placed_leds(&[None, None, None]);
    let query = LedQ::names(vec!["led2", "led0", "led1"]);
    let placed = place_leds(Viewport::Sequence, &context.base.leds, None, &query, DEFAULT_LED_RADIUS);
    let names: Vec<_> = placed.iter().map(|(led, _)| led.name.as_str()).collect();
    let spots: Vec<_> = placed.iter().map(|(_, sample)| sample.pos.x).collect();
    assert_eq!(names, ["led2", "led0", "led1"]);
    assert_eq!(spots, [0.0, 0.5, 1.0]);
  }

  #[test]
  fn place_leds_puts_an_area_on_the_plane_with_the_radius_its_given() {
    let (context, _) = placed_leds(&[at(2.0, 3.0), at(50.0, 3.0), None]);
    let query = LedQ::every();
    let placed = place_leds(Viewport::Area, &context.base.leds, Some(&PLANE), &query, 0.25);
    assert_eq!(placed.len(), 2);
    assert_eq!(placed[0].0.name, "led0");
    assert_eq!(placed[0].1, Sample::new(Vec2::new(2.0, 3.0), 0.25));
    assert_eq!(placed[1].1, Sample::new(Vec2::new(50.0, 3.0), 0.25));
  }

  #[test]
  fn place_leds_reaches_nothing_on_an_area_without_a_plane() {
    let (context, _) = placed_leds(&[at(2.0, 3.0)]);
    let query = LedQ::every();
    assert!(place_leds(Viewport::Area, &context.base.leds, None, &query, DEFAULT_LED_RADIUS).is_empty());
  }

  #[test]
  fn stop_clears_leds_on_their_layer() {
    let (context, addresses) = placed_leds(&[at(2.0, 2.0)]);
    let mut program = LedPlayback::new(LedScene::new(Viewport::Area, LedTargets::Single(LedTarget::new(LedQ::every()).on_plane("Plane").at_z(3)), red())).unwrap();

    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));

    program.stop(&context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], None);
  }

  #[test]
  fn each_z_is_its_own_layer() {
    let (context, addresses) = three_leds();
    let solid = |color: Rgba<u8>| scene(FillNode::Solid(Color(color).into()));
    let mut under = on(LedQ::name("led1"), solid(Rgba::red()));
    let target = LedTarget::new(LedQ::name("led1")).at_z(1);
    let mut over = LedPlayback::new(LedScene::new(Viewport::Sequence, LedTargets::Single(target), solid(Rgba::blue()))).unwrap();
    let layers = || context.sys_ctx().systems.expect::<LedSystem>().declarations_for(&addresses[0]).len();

    under.apply(Duration::from_millis(16), &context.sys_ctx());
    over.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(layers(), 2);

    over.stop(&context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));
  }

  fn on_two_targets() -> GfxComposition {
    let solid = |color: Rgba<u8>| Fill::new(FillNode::Solid(Color(color).into()));
    GfxComposition::new(vec![
      Layer::new(solid(Rgba::red())).targets(["left"]),
      Layer::new(solid(Rgba::blue())).targets(["right"]),
    ])
    .timing(Timing::new(1.0, Cycle::Once))
  }

  fn left_and_right(left: &str, right: &str) -> LedPlayback {
    let targets = vec![("left", LedTarget::new(LedQ::name(left))), ("right", LedTarget::new(LedQ::name(right)).at_z(2))];
    on_targets(Viewport::Sequence, targets, on_two_targets())
  }

  #[test]
  fn targets_play_their_own_layers_on_their_own_leds() {
    let (context, addresses) = three_leds();
    let program = left_and_right("led1", "led3");
    assert_eq!(played(&context, &addresses, program), vec![Some(Rgba::red()), None, Some(Rgba::blue())]);
  }

  #[test]
  fn targets_run_on_one_clock_and_clear_together() {
    let (context, addresses) = three_leds();
    let mut program = left_and_right("led1", "led2");

    program.apply(Duration::from_secs(1), &context.sys_ctx());
    assert!(program.is_complete());
    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses), vec![None, None, None]);
  }

  #[test]
  fn each_target_spreads_over_its_own_leds() {
    let (context, addresses) = three_leds();
    let targets = vec![("a", LedTarget::new(LedQ::names(vec!["led1", "led2"]))), ("b", LedTarget::new(LedQ::name("led3")))];
    let program = on_targets(Viewport::Sequence, targets, scene(red_to_blue()));

    // on its own, the third LED is the start of its target
    assert_eq!(
      played(&context, &addresses, program),
      vec![Some(Rgba::red()), Some(Rgba::blue()), Some(Rgba::red())]
    );
  }

  #[test]
  fn stop_clears_every_target() {
    let (context, addresses) = three_leds();
    let mut program = left_and_right("led1", "led2");
    program.apply(Duration::from_millis(16), &context.sys_ctx());

    program.stop(&context.sys_ctx());
    assert_eq!(colors(&context, &addresses), vec![None, None, None]);
  }

  #[test]
  fn targets_work_on_a_plane_too() {
    let (context, addresses) = placed_leds(&[at(0.0, 5.0), at(10.0, 5.0)]);
    let targets = vec![("a", LedTarget::new(LedQ::name("led1")).on_plane("Plane"))];
    let program = on_targets(Viewport::Area, targets, scene(red_to_blue()));
    assert_eq!(played(&context, &addresses, program), vec![None, Some(Rgba::blue())]);
  }

  #[test]
  fn with_leds_moves_a_single_target() {
    let (context, addresses) = three_leds();
    let target = LedTarget::new(LedQ::name("led1")).at_z(3);
    let scene = LedScene::new(Viewport::Sequence, LedTargets::Single(target), timed(Cycle::Forever));
    let program = LedPlayback::with_leds(scene, LedQ::name("led2")).unwrap();

    assert_eq!(played(&context, &addresses, program), vec![None, Some(Rgba::red()), None]);
  }

  #[test]
  fn a_scene_with_no_targets_cant_play() {
    let scene = LedScene::new(Viewport::Sequence, LedTargets::None, timed(Cycle::Once));
    assert!(matches!(LedPlayback::new(scene.clone()), Err(PlaybackError::NoTargets)));
    assert!(matches!(LedPlayback::with_leds(scene, LedQ::every()), Err(PlaybackError::NoTargets)));
  }

  #[test]
  fn several_targets_cant_be_moved() {
    let targets = IndexMap::from([("a".to_string(), LedTarget::new(LedQ::every()))]);
    let scene = LedScene::new(Viewport::Sequence, LedTargets::Multiple(targets), timed(Cycle::Once));
    assert!(matches!(LedPlayback::with_leds(scene, LedQ::every()), Err(PlaybackError::MultipleTargets)));
  }

  #[test]
  fn a_tag_finds_the_leds_that_have_it() {
    let (mut context, addresses) = three_leds();
    let address = LedAddress::new(ExpAddress::default(), 9);
    context.insert_led(LED {
      name: "tagged".into(),
      address: address.clone(),
      tags: vec![Box::new(frontbox_core::tags::Playfield)],
      ..Default::default()
    });
    let mut program = on(LedQ::tag::<frontbox_core::tags::Playfield>(), scene(FillNode::Solid(Color(Rgba::red()).into())));

    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &[address])[0], Some(Rgba::red()));
    assert_eq!(colors(&context, &addresses)[0], None);
  }
}
