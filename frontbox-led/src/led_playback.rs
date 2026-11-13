use std::collections::HashMap;
use std::sync::Arc;
use std::fmt;

use frontbox_core::prelude::*;
use frontbox_gfx::prelude::{Rgba, RgbaColor};
use frontbox_gfx::layout::Line;
use frontbox_gfx::scene::{self, FrameContext, GfxComposition, Graph, Images, Params, Sample, Value, Visits};

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
///   layers: [(
///     leds: Name("lane"),
///     surface: Sequence,
///     layer: (fill: (base: Solid(Param("color")))),
///   )],
/// )"#).unwrap();
///
/// let program = LedPlayback::new(scene).unwrap().param("color", Rgba::red());
/// ```
///
/// A scene whose layers are all on the same LEDs can be moved to others, for when which ones is only known in the game:
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # use frontbox_led::prelude::*;
/// # let scene = LedScene::load(r#"Led(
/// #   version: 1,
/// #   layers: [(
/// #     leds: Name("lane"),
/// #     surface: Sequence,
/// #     layer: (fill: (base: Solid(Fixed(255, 0, 0, 255)))),
/// #   )],
/// # )"#).unwrap();
/// let program = LedPlayback::with_leds(scene, LedQ::name("other_lane")).unwrap();
/// ```
#[derive(Clone)]
pub struct LedPlayback {
  scene: LedScene,
  /// Where each z's layers land, found the first time it plays as LEDs don't move
  placements: Option<Vec<Placement>>,
  params: Params,
  images: Images,
  elapsed: Duration,
  /// How much of the scene plays in a second
  speed: f32,
  playback_seed: u32,
  playing: bool,
  end_behavior: EndBehavior,
  undeclared: bool,
}

impl LedPlayback {
  /// A scene with a layer that isn't on any LEDs yet is refused
  pub fn new(scene: LedScene) -> Result<Self, PlaybackError> {
    if scene.layers.iter().any(|layer| layer.leds.is_none()) {
      return Err(PlaybackError::NoLeds);
    }
    Ok(Self {
      scene,
      placements: None,
      params: Params::new(),
      images: Images::new(),
      elapsed: Duration::ZERO,
      speed: 1.0,
      playback_seed: rand::random(),
      playing: true,
      end_behavior: EndBehavior::Clear,
      undeclared: false,
    })
  }

  /// Plays a scene on other LEDs. Each layer's z and surface stay as the scene says. That's only for a scene whose layers
  /// are all on the same LEDs, as there's otherwise no saying which of them to move.
  pub fn with_leds(mut scene: LedScene, leds: LedQ) -> Result<Self, PlaybackError> {
    if scene.layers.iter().any(|layer| layer.leds.is_none()) {
      return Err(PlaybackError::NoLeds);
    }
    let first = scene.layers.first().and_then(|layer| layer.leds.as_ref());
    if scene.layers.iter().any(|layer| layer.leds.as_ref() != first) {
      return Err(PlaybackError::DifferentLeds);
    }
    for layer in &mut scene.layers {
      layer.leds = Some(leds.clone());
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

  /// Plays a cycle of the scene in `duration`, however long it was written to playback_seed, for when how long it lasts is only
  /// known once it's played
  pub fn over(mut self, duration: Duration) -> Self {
    if let Some(timing) = self.scene.timing {
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

    let time = self.elapsed.as_secs_f32();
    let scene = &self.scene;
    for placement in self.placements.get_or_insert_with(|| place(scene, ctx)) {
      let colors = placement.colors(time, self.playback_seed, &self.params, &self.images);

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
    self.playback_seed = rand::random();
    self.undeclared = false;
  }

  pub fn is_playing(&self) -> bool {
    self.playing
  }

  /// Whether the scene has played through. A scene that's static or repeats forever never completes, and plays until
  /// it's stopped.
  pub fn is_complete(&self) -> bool {
    self.scene.timing.is_some_and(|timing| timing.is_complete(self.elapsed.as_secs_f32()))
  }

  /// How long the scene takes to play through, at the speed it's played at. `None` for a scene that never completes:
  /// one that's static or repeats forever.
  pub fn length(&self) -> Option<Duration> {
    let length = self.scene.timing?.length()?;
    Some(Duration::from_secs_f32(length / self.speed))
  }
}

/// Why a scene can't be played as it was asked to
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackError {
  /// A layer of the scene isn't on any LEDs
  NoLeds,
  /// Code can only change the LEDs of a scene whose layers are all on the same ones
  DifferentLeds,
}

impl fmt::Display for PlaybackError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NoLeds => write!(f, "a layer of the scene has no LEDs to play on"),
      Self::DifferentLeds => write!(f, "a scene with layers on different LEDs can't be moved to other LEDs"),
    }
  }
}

impl std::error::Error for PlaybackError {}

/// The LEDs matching `query` that a scene on `surface` lands on, each with where it samples the scene. This is what
/// [`LedPlayback`] does, for anything that wants to know what it would show without playing it.
///
/// On a `Sequence` they spread along it in order. An `Area` needs a `plane` to put them on, and without one reaches no
/// LEDs.
pub fn place_leds<'a>(
  surface: scene::Surface,
  leds: &'a LedLookup,
  plane: Option<&ReferencePlane>,
  query: &'a LedQ,
  radius: f32,
) -> Vec<(&'a LED, Sample)> {
  match (surface, plane) {
    (scene::Surface::Sequence, _) => {
      let matching: Vec<_> = leds.query_iter(query).collect();
      let line = Line::new(matching.len());
      matching.into_iter().zip(line.samples()).collect()
    }
    (scene::Surface::Area, Some(plane)) => place_on_plane(leds, plane, query, radius),
    (scene::Surface::Area, None) => Vec::new(),
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

/// Where the scene's layers land, with those on one z together, as an LED only has one declaration there
fn place(scene: &LedScene, ctx: &SystemContext) -> Vec<Placement> {
  let composition = scene.composition();
  let mut placements: Vec<Placement> = Vec::new();
  for (index, layer) in scene.layers.iter().enumerate() {
    let found = placements.iter().position(|placement| placement.ids.z_index == layer.z);
    let at = found.unwrap_or_else(|| {
      placements.push(Placement::new(layer.z));
      placements.len() - 1
    });
    placements[at].add(composition.only(index), layer, ctx);
  }
  placements
}

/// What's declared on one z: every LED its layers land on, and each layer's samples of them
#[derive(Clone)]
struct Placement {
  ids: LedIdentifications,
  /// Where each LED is in `ids`
  spots: HashMap<LedAddress, usize>,
  parts: Vec<Part>,
}

impl Placement {
  fn new(z: i8) -> Self {
    Self { ids: LedIdentifications::new(Vec::new(), z), spots: HashMap::new(), parts: Vec::new() }
  }

  fn add(&mut self, composition: GfxComposition, layer: &LedLayer, ctx: &SystemContext) {
    let plane = layer.surface.plane().and_then(|plane| {
      let found = ctx.leds.plane(plane);
      if found.is_none() {
        log::error!("LedPlayback: there's no plane \"{plane}\" added to the app");
      }
      found
    });
    let surface = layer.surface.scene_surface();
    let placed = layer.leds.as_ref().map_or(Vec::new(), |leds| place_leds(surface, &ctx.leds, plane, leds, ctx.led_radius));
    let (placed, graph) = match &layer.layer.graph {
      Some(_) => {
        let (placed, graph) = place_graph(placed, plane);
        (placed, Some(graph))
      }
      None => (placed, None),
    };
    let samples = placed.into_iter().map(|(led, sample)| (self.spot(&led.address), sample)).collect();
    let size = plane.map_or(Vec2::ZERO, |plane| plane.extent);
    self.parts.push(Part { composition, surface, size, samples, graph, visits: None });
  }

  fn spot(&mut self, led: &LedAddress) -> usize {
    *self.spots.entry(led.clone()).or_insert_with(|| {
      self.ids.leds.push(led.clone());
      self.ids.leds.len() - 1
    })
  }

  /// The color of each LED in `ids`, with the layers on it drawn bottom to top
  fn colors(&mut self, time: f32, playback_seed: u32, params: &Params, images: &Images) -> Vec<Rgba<u8>> {
    let mut colors = vec![Rgba([0, 0, 0, 0]); self.ids.leds.len()];
    for part in &mut self.parts {
      let frame = FrameContext { time, playback_seed, params, images, size: part.size, surface: part.surface, visits: None };
      part.update_visits(&frame);
      let frame = FrameContext { visits: part.visits.as_ref().map(|(_, visits)| visits), ..frame };
      for (spot, sample) in &part.samples {
        colors[*spot] = part.composition.sample(sample, &frame).composite_over(colors[*spot]);
      }
    }
    colors
  }
}

/// One layer on its surface
#[derive(Clone)]
struct Part {
  composition: GfxComposition,
  surface: scene::Surface,
  /// The size of what it's on
  size: Vec2,
  /// Its LEDs, by their spot in the [`Placement`], and where each samples it
  samples: Vec<(usize, Sample)>,
  /// For a layer with a graph, the one built from its LEDs
  graph: Option<Arc<Graph>>,
  /// The traversals of the cycle being played, kept until the cycle, playback seed, or params change
  visits: Option<(VisitsFor, Visits)>,
}

impl Part {
  /// Works out the traversals for this frame's cycle, for a layer with a graph, if they aren't kept already
  fn update_visits(&mut self, frame: &FrameContext) {
    let Some(graph) = &self.graph else {
      return;
    };
    let cycle = self.composition.timing.map_or(0, |timing| timing.cycle_index(frame.time));
    let current = VisitsFor { cycle, playback_seed: frame.playback_seed, params: frame.params.clone() };
    if self.visits.as_ref().is_none_or(|(kept, _)| *kept != current) {
      self.visits = Some((current, self.composition.visits(graph, frame)));
    }
  }
}

/// What a cycle's visits were worked out for
#[derive(Clone, PartialEq)]
struct VisitsFor {
  cycle: u32,
  playback_seed: u32,
  params: Params,
}

#[cfg(test)]
mod tests {
  use std::sync::LazyLock;

  use frontbox_gfx::animation::{Curve, Cycle};
  use frontbox_gfx::scene::*;
  use frontbox_gfx::prelude::*;

  use super::*;
  use crate::Surface;

  static PLANE: ReferencePlane = ReferencePlane::new("Plane").extent(Vec2::new(10.0, 10.0)).build();
  static WIDE: ReferencePlane = ReferencePlane::new("Wide").extent(Vec2::new(20.0, 10.0)).build();
  static OFFSET: LazyLock<ReferencePlane> =
    LazyLock::new(|| ReferencePlane::new("Offset").origin(Vec3::new(100.0, 0.0, 0.0)).extent(Vec2::new(10.0, 10.0)).build());

  /// Every layer of a composition on the same LEDs and surface
  fn scene_on(leds: LedQ, surface: Surface, composition: GfxComposition) -> LedScene {
    let GfxComposition { params, timing, layers } = composition;
    let layers = layers.into_iter().map(|layer| LedLayer { leds: Some(leds.clone()), z: 0, surface: surface.clone(), layer }).collect();
    LedScene { params, timing, ..LedScene::new(layers) }
  }

  /// Plays along some LEDs
  fn on(leds: LedQ, composition: GfxComposition) -> LedPlayback {
    LedPlayback::new(scene_on(leds, Surface::Sequence, composition)).unwrap()
  }

  /// Plays across a plane
  fn across(plane: &str, leds: LedQ, composition: GfxComposition) -> LedPlayback {
    LedPlayback::new(scene_on(leds, Surface::Area(plane.to_string()), composition)).unwrap()
  }

  fn on_plane(leds: LedQ, composition: GfxComposition) -> LedPlayback {
    across("Plane", leds, composition)
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
  fn length_is_how_long_the_scene_takes_to_play_through() {
    let leds = || LedQ::name("led1");
    assert_eq!(on(leds(), timed(Cycle::Times(3))).length(), Some(Duration::from_secs(3)));
    assert_eq!(on(leds(), timed(Cycle::Once)).over(Duration::from_secs(20)).length(), Some(Duration::from_secs(20)));
    assert_eq!(on(leds(), timed(Cycle::Forever)).length(), None);
    assert_eq!(on(leds(), scene(FillNode::Solid(Color(Rgba::red()).into()))).length(), None, "static");
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
    let scene = GfxComposition::new(vec![Layer::new(fill).region(red_circle(Vec2::new(11.0, 5.0).into()))]);
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
    let scene = GfxComposition::new(vec![Layer::new(fill).region(red_circle(Prop::Track(path)))])
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
      let circle = Layer::new(Fill::new(FillNode::Solid(Color(Rgba::red()).into()))).region(
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
    let placed = place_leds(scene::Surface::Sequence, &context.base.leds, None, &query, DEFAULT_LED_RADIUS);
    let names: Vec<_> = placed.iter().map(|(led, _)| led.name.as_str()).collect();
    let spots: Vec<_> = placed.iter().map(|(_, sample)| sample.pos.x).collect();
    assert_eq!(names, ["led2", "led0", "led1"]);
    assert_eq!(spots, [0.0, 0.5, 1.0]);
  }

  #[test]
  fn place_leds_puts_an_area_on_the_plane_with_the_radius_its_given() {
    let (context, _) = placed_leds(&[at(2.0, 3.0), at(50.0, 3.0), None]);
    let query = LedQ::every();
    let placed = place_leds(scene::Surface::Area, &context.base.leds, Some(&PLANE), &query, 0.25);
    assert_eq!(placed.len(), 2);
    assert_eq!(placed[0].0.name, "led0");
    assert_eq!(placed[0].1, Sample::new(Vec2::new(2.0, 3.0), 0.25));
    assert_eq!(placed[1].1, Sample::new(Vec2::new(50.0, 3.0), 0.25));
  }

  #[test]
  fn place_leds_reaches_nothing_on_an_area_without_a_plane() {
    let (context, _) = placed_leds(&[at(2.0, 3.0)]);
    let query = LedQ::every();
    assert!(place_leds(scene::Surface::Area, &context.base.leds, None, &query, DEFAULT_LED_RADIUS).is_empty());
  }

  #[test]
  fn stop_clears_leds_on_their_layer() {
    let (context, addresses) = placed_leds(&[at(2.0, 2.0)]);
    let layer = LedLayer::area(LedQ::every(), "Plane", solid_layer(Rgba::red())).at_z(3);
    let mut program = LedPlayback::new(LedScene::new(vec![layer])).unwrap();

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
    let mut over = LedPlayback::new(LedScene::new(vec![LedLayer::sequence(LedQ::name("led1"), solid_layer(Rgba::blue())).at_z(1)])).unwrap();
    let layers = || context.sys_ctx().systems.expect::<LedSystem>().declarations_for(&addresses[0]).len();

    under.apply(Duration::from_millis(16), &context.sys_ctx());
    over.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(layers(), 2);

    over.stop(&context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));
  }

  fn solid_layer(color: Rgba<u8>) -> Layer {
    Layer::new(Fill::new(FillNode::Solid(Color(color).into())))
  }

  fn see_through(color: Rgba<u8>) -> Layer {
    solid_layer(Rgba([color[0], color[1], color[2], 128]))
  }

  fn two_layers(under: &str, over: &str) -> LedPlayback {
    LedPlayback::new(
      LedScene::new(vec![
        LedLayer::sequence(LedQ::name(under), solid_layer(Rgba::red())),
        LedLayer::sequence(LedQ::name(over), solid_layer(Rgba::blue())).at_z(2),
      ])
      .timing(Timing::new(1.0, Cycle::Once)),
    )
    .unwrap()
  }

  #[test]
  fn layers_play_on_their_own_leds() {
    let (context, addresses) = three_leds();
    let program = two_layers("led1", "led3");
    assert_eq!(played(&context, &addresses, program), vec![Some(Rgba::red()), None, Some(Rgba::blue())]);
  }

  #[test]
  fn layers_run_on_one_clock_and_clear_together() {
    let (context, addresses) = three_leds();
    let mut program = two_layers("led1", "led2");

    program.apply(Duration::from_secs(1), &context.sys_ctx());
    assert!(program.is_complete());
    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses), vec![None, None, None]);
  }

  #[test]
  fn each_layer_spreads_over_its_own_leds() {
    let (context, addresses) = three_leds();
    let fade = || Layer::new(Fill::new(red_to_blue()));
    let scene = LedScene::new(vec![LedLayer::sequence(LedQ::names(vec!["led1", "led2"]), fade()), LedLayer::sequence(LedQ::name("led3"), fade())]);

    // on its own, the third LED is the start of its layer
    assert_eq!(
      played(&context, &addresses, LedPlayback::new(scene).unwrap()),
      vec![Some(Rgba::red()), Some(Rgba::blue()), Some(Rgba::red())]
    );
  }

  #[test]
  fn stop_clears_every_layer() {
    let (context, addresses) = three_leds();
    let mut program = two_layers("led1", "led2");
    program.apply(Duration::from_millis(16), &context.sys_ctx());

    program.stop(&context.sys_ctx());
    assert_eq!(colors(&context, &addresses), vec![None, None, None]);
  }

  #[test]
  fn each_layer_is_measured_against_its_own_plane() {
    // x = 10 is halfway along Wide, and x = 110 is the far edge of Offset, which starts at x = 100
    let (context, addresses) = placed_leds(&[at(10.0, 5.0), at(110.0, 5.0)]);
    let gradient = |leds: &str, plane: &str| LedLayer::area(LedQ::name(leds), plane, Layer::new(Fill::new(red_to_blue())));
    let scene = LedScene::new(vec![gradient("led0", "Wide"), gradient("led1", "Offset")]);

    let shown = played(&context, &addresses, LedPlayback::new(scene).unwrap());
    assert_eq!(shown, vec![Some(Rgba([127, 0, 128, 255])), Some(Rgba::blue())]);
  }

  #[test]
  fn a_scene_can_mix_a_sequence_with_an_area() {
    let (context, addresses) = placed_leds(&[at(5.0, 5.0), None]);
    let scene = LedScene::new(vec![
      LedLayer::area(LedQ::name("led0"), "Plane", solid_layer(Rgba::red())),
      LedLayer::sequence(LedQ::name("led1"), solid_layer(Rgba::blue())),
    ]);
    assert_eq!(played(&context, &addresses, LedPlayback::new(scene).unwrap()), vec![Some(Rgba::red()), Some(Rgba::blue())]);
  }

  #[test]
  fn layers_on_one_z_blend_rather_than_replace_each_other() {
    let (context, addresses) = three_leds();
    let scene = LedScene::new(vec![
      LedLayer::sequence(LedQ::name("led1"), solid_layer(Rgba::blue())),
      LedLayer::sequence(LedQ::name("led1"), see_through(Rgba::red())),
    ]);
    let shown = played(&context, &addresses, LedPlayback::new(scene).unwrap())[0].unwrap();
    assert!(shown[0] > 100 && shown[2] > 100, "{shown:?}");
    assert_eq!(shown[3], 255);
  }

  #[test]
  fn a_layer_that_misses_an_led_leaves_it_to_the_others_on_its_z() {
    let (context, addresses) = placed_leds(&[at(5.0, 5.0)]);
    let far_away = Shape::new(ShapeNode::circle(1.0)).transform(Transform::Translate(Vec2::new(50.0, 50.0).into()));
    let scene = LedScene::new(vec![
      LedLayer::area(LedQ::every(), "Plane", solid_layer(Rgba::red())),
      LedLayer::area(LedQ::every(), "Plane", solid_layer(Rgba::blue()).region(far_away)),
    ]);
    assert_eq!(played(&context, &addresses, LedPlayback::new(scene).unwrap()), vec![Some(Rgba::red())]);
  }

  #[test]
  fn layers_on_different_zs_are_declared_apart() {
    let (context, addresses) = three_leds();
    let scene = LedScene::new(vec![
      LedLayer::sequence(LedQ::name("led1"), solid_layer(Rgba::red())),
      LedLayer::sequence(LedQ::name("led1"), solid_layer(Rgba::blue())).at_z(1),
    ]);
    played(&context, &addresses, LedPlayback::new(scene).unwrap());
    assert_eq!(context.sys_ctx().systems.expect::<LedSystem>().declarations_for(&addresses[0]).len(), 2);
  }

  fn stacked(leds: LedQ) -> LedScene {
    LedScene::new(vec![LedLayer::sequence(leds.clone(), solid_layer(Rgba::red())), LedLayer::sequence(leds, see_through(Rgba::red())).at_z(3)])
  }

  #[test]
  fn with_leds_moves_every_layer_keeping_its_z() {
    let (context, addresses) = three_leds();
    let program = LedPlayback::with_leds(stacked(LedQ::name("led1")), LedQ::name("led2")).unwrap();

    assert_eq!(played(&context, &addresses, program)[0], None);
    assert_eq!(context.sys_ctx().systems.expect::<LedSystem>().declarations_for(&addresses[1]).len(), 2);
  }

  #[test]
  fn a_layer_with_no_leds_cant_play() {
    let mut scene = stacked(LedQ::every());
    scene.layers[1].leds = None;
    assert!(matches!(LedPlayback::new(scene.clone()), Err(PlaybackError::NoLeds)));
    assert!(matches!(LedPlayback::with_leds(scene, LedQ::every()), Err(PlaybackError::NoLeds)));
  }

  #[test]
  fn layers_on_different_leds_cant_be_moved() {
    let mut scene = stacked(LedQ::every());
    scene.layers[1].leds = Some(LedQ::name("led1"));
    assert!(LedPlayback::new(scene.clone()).is_ok());
    assert!(matches!(LedPlayback::with_leds(scene, LedQ::every()), Err(PlaybackError::DifferentLeds)));
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

#[cfg(test)]
mod graph_tests {
  use frontbox_gfx::animation::Cycle;
  use frontbox_gfx::scene::*;
  use frontbox_gfx::prelude::*;

  use super::*;
  use crate::Surface;

  static PLANE: ReferencePlane = ReferencePlane::new("Plane").extent(Vec2::new(10.0, 10.0)).build();

  /// LEDs "led0".. at these places, on a 10 by 10 plane
  fn leds(locations: &[Vec3]) -> (TestContext, Vec<LedAddress>) {
    let mut context = TestContext::default();
    context.insert_system(LedSystem::new());
    let addresses: Vec<_> = (0..locations.len()).map(|i| LedAddress::new(ExpAddress::default(), i as u8)).collect();
    for (i, address) in addresses.iter().enumerate() {
      context.insert_led(LED { name: format!("led{i}"), address: address.clone(), location: Some(locations[i]), ..Default::default() });
    }
    context.add_plane(&PLANE);
    (context, addresses)
  }

  /// Four LEDs in a row on the plane, one apart
  fn row() -> (TestContext, Vec<LedAddress>) {
    leds(&(0..4).map(|x| Vec3::new(x as f32 + 1.0, 5.0, 0.0)).collect::<Vec<_>>())
  }

  /// Red where traversal "wave" has just been: one edge a second out from `from`, fading over a step
  fn wave(from: &str, surface: Surface) -> LedPlayback {
    let shown = Prop::Visit(VisitResponse { smooth_out: Some(1.0), ..VisitResponse::new("wave", 0.0, 1.0) });
    let wave = Traversal::new("wave", NodeRef::name(from), TraversalAlgorithm::BreadthFirst { wander: 0.0.into() }, 1.0);
    let fill = Fill::new(FillNode::Solid(Color(Rgba::red()).into())).adjust(ColorAdjust::Opacity(shown));
    let layer = LedLayer { leds: Some(LedQ::every()), z: 0, surface, layer: Layer::new(fill).graph(GraphMask::new(vec![wave])) };
    LedPlayback::new(LedScene::new(vec![layer]).timing(Timing::new(10.0, Cycle::Once))).unwrap()
  }

  fn lit(context: &TestContext, addresses: &[LedAddress]) -> Vec<bool> {
    let ctx = context.sys_ctx();
    let led_system = ctx.systems.expect::<LedSystem>();
    addresses.iter().map(|address| led_system.declarations_for(address).first().is_some_and(|d| d.color[3] > 127)).collect()
  }

  #[test]
  fn a_traversal_moves_across_the_leds_it_reaches() {
    let (context, addresses) = row();
    let mut program = wave("led0", Surface::Area("Plane".into()));
    program.apply(Duration::ZERO, &context.sys_ctx());
    assert_eq!(lit(&context, &addresses), [true, false, false, false]);
    program.apply(Duration::from_secs(2), &context.sys_ctx());
    assert_eq!(lit(&context, &addresses), [false, false, true, false]);
  }

  #[test]
  fn leds_off_the_plane_are_left_out_of_its_graph() {
    // the third is raised above the plane, so the wave can't go through it to the fourth
    let (context, addresses) = leds(&[Vec3::new(1.0, 5.0, 0.0), Vec3::new(2.0, 5.0, 0.4), Vec3::new(3.0, 5.0, 2.0), Vec3::new(4.0, 5.0, 0.0)]);
    let mut program = wave("led0", Surface::Area("Plane".into()));
    program.apply(Duration::from_secs(1), &context.sys_ctx());
    assert_eq!(lit(&context, &addresses), [false, true, false, false]);
    program.apply(Duration::from_secs(1), &context.sys_ctx());
    assert_eq!(lit(&context, &addresses), [false, false, false, true]);
  }

  #[test]
  fn a_sequence_is_a_chain_in_its_order() {
    let (context, addresses) = row();
    let mut program = wave("led3", Surface::Sequence);
    program.apply(Duration::from_secs(1), &context.sys_ctx());
    assert_eq!(lit(&context, &addresses), [false, false, true, false]);
  }

  #[test]
  fn the_same_leds_share_a_graph() {
    let (context, _) = row();
    let ctx = context.sys_ctx();
    let graph = || {
      let every = LedQ::every();
      let placed = place_leds(scene::Surface::Area, &ctx.leds, Some(&PLANE), &every, DEFAULT_LED_RADIUS);
      place_graph(placed, Some(&PLANE)).1
    };
    assert!(Arc::ptr_eq(&graph(), &graph()));
    assert_eq!(graph().edges().count(), 3);
  }
}
