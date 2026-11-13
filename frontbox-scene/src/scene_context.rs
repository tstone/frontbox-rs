use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use frontbox_core::prelude::*;
use frontbox_gfx::scene::{Images, Params, Value};

use crate::*;

/// A [`Scene`] and how to play it: its parameters, images, and how long it takes. It holds nothing about a scene that's
/// playing, so one can be kept and played any number of times. A `Scene` converts into one that plays as written.
///
/// Parameters are how code changes a scene:
///
/// ```rust
/// # use frontbox_scene::*;
/// # use frontbox_gfx::prelude::*;
/// let scene = Scene::load(r#"Scene(
///   version: 2,
///   layers: [Leds(leds: Name("lane"), surface: Sequence, layer: (fill: (base: Solid(Param("color")))))],
/// )"#).unwrap();
///
/// let scene = SceneContext::new(scene).param("color", Rgba::red());
/// ```
///
/// What needs a scene to complete, such as [`play_scene`](SceneSystemExt::play_scene), takes a
/// `SceneContext<Completes>`, which only [`SceneContext::completing`] makes:
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # use frontbox_scene::*;
/// let scene = Scene::load(r#"Scene(
///   version: 2,
///   timing: (duration: 0.5),
///   layers: [Leds(leds: Name("lane"), surface: Sequence, layer: (fill: (base: Solid(Fixed(255, 0, 0, 255)))))],
/// )"#).unwrap();
///
/// let scene = SceneContext::completing(scene).unwrap();
/// assert_eq!(scene.length(), Duration::from_millis(500));
/// ```
#[derive(Debug, Clone)]
pub struct SceneContext<C = Unchecked> {
  scene: Arc<Scene>,
  params: Params,
  images: Images,
  /// How much of the scene plays in a second
  speed: f32,
  end_behavior: EndBehavior,
  completes: PhantomData<C>,
}

impl SceneContext {
  pub fn new(scene: impl Into<Arc<Scene>>) -> Self {
    Self::unchecked(scene.into())
  }

  /// What to do when the scene is over. It clears unless told otherwise.
  pub fn end_behavior(mut self, end_behavior: EndBehavior) -> Self {
    self.end_behavior = end_behavior;
    self
  }

  /// How long the scene takes to play through, at the speed it's played at. `None` for a scene that never completes:
  /// one that's static or repeats forever.
  pub fn length(&self) -> Option<Duration> {
    Some(Duration::from_secs_f32(self.scene_length()? / self.speed))
  }
}

impl SceneContext<Completes> {
  /// For a scene that plays through and completes. One that's static or repeats forever is refused.
  pub fn completing(scene: impl Into<Arc<Scene>>) -> Result<Self, NeverCompletes> {
    let context = SceneContext::unchecked(scene.into());
    match context.scene_length() {
      Some(_) => Ok(context),
      None => Err(NeverCompletes),
    }
  }

  /// How long the scene takes to play through, at the speed it's played at
  pub fn length(&self) -> Duration {
    // checked to have a length by `completing`
    Duration::from_secs_f32(self.scene_length().unwrap_or_default() / self.speed)
  }
}

impl<C> SceneContext<C> {
  fn unchecked(scene: Arc<Scene>) -> Self {
    Self { scene, params: Params::new(), images: Images::new(), speed: 1.0, end_behavior: EndBehavior::Clear, completes: PhantomData }
  }

  /// Sets a parameter, taking the place of the scene's default
  pub fn param(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
    self.set_param(name, value);
    self
  }

  /// [`SceneContext::param`] for one that's already built
  pub fn set_param(&mut self, name: impl Into<String>, value: impl Into<Value>) {
    self.params.insert(name.into(), value.into());
  }

  /// The images the scene draws, from [`Content::images_for`](frontbox_gfx::scene::Content::images_for). That reads
  /// files, so do it ahead of time rather than in a tick.
  pub fn images(mut self, images: Images) -> Self {
    self.images = images;
    self
  }

  /// Plays a cycle of the scene in `duration`, however long it was written to be, for when how long it lasts is only
  /// known in the game. A zero duration is ignored, as it is on a static scene.
  pub fn over(mut self, duration: Duration) -> Self {
    if let Some(timing) = self.scene.timing
      && !duration.is_zero()
    {
      self.speed = timing.duration / duration.as_secs_f32();
    }
    self
  }

  /// Plays the scene's LED layers on `leds`, for when which LEDs is only known in the game. See
  /// [`Scene::moved_to_leds`].
  pub fn on_leds(mut self, leds: LedQ) -> Result<Self, DifferentLeds> {
    self.scene = Arc::new(Arc::unwrap_or_clone(self.scene).moved_to_leds(leds)?);
    Ok(self)
  }

  pub fn scene(&self) -> &Arc<Scene> {
    &self.scene
  }

  pub(crate) fn params(&self) -> &Params {
    &self.params
  }

  pub(crate) fn scene_images(&self) -> &Images {
    &self.images
  }

  pub(crate) fn speed(&self) -> f32 {
    self.speed
  }

  pub(crate) fn scene_end_behavior(&self) -> EndBehavior {
    self.end_behavior.clone()
  }

  /// Seconds for every cycle to play as written, or `None` if that never happens
  fn scene_length(&self) -> Option<f32> {
    self.scene.timing?.length()
  }
}

impl From<Scene> for SceneContext {
  fn from(scene: Scene) -> Self {
    Self::new(scene)
  }
}

impl From<Arc<Scene>> for SceneContext {
  fn from(scene: Arc<Scene>) -> Self {
    Self::new(scene)
  }
}

impl From<SceneContext<Completes>> for SceneContext {
  fn from(context: SceneContext<Completes>) -> Self {
    Self {
      scene: context.scene,
      params: context.params,
      images: context.images,
      speed: context.speed,
      end_behavior: context.end_behavior,
      completes: PhantomData,
    }
  }
}

/// Marks a [`SceneContext`] whose scene may or may not complete
#[derive(Debug, Clone, Copy)]
pub struct Unchecked;

/// Marks a [`SceneContext`] whose scene plays through and completes
#[derive(Debug, Clone, Copy)]
pub struct Completes;

/// Why [`SceneContext::completing`] refused: the scene is static or repeats forever
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NeverCompletes;

impl fmt::Display for NeverCompletes {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "the scene never completes. Give it a timing that plays a set number of times")
  }
}

impl std::error::Error for NeverCompletes {}

#[cfg(test)]
mod tests {
  use frontbox_gfx::animation::Cycle;
  use frontbox_gfx::scene::Timing;

  use super::*;

  fn timed(duration: f32, cycle: Cycle) -> Scene {
    Scene::new(vec![]).timing(Timing::new(duration, cycle))
  }

  #[test]
  fn a_scene_that_plays_a_set_number_of_times_completes() {
    let context = SceneContext::completing(timed(0.5, Cycle::Times(2))).unwrap();
    assert_eq!(context.length(), Duration::from_secs(1));
  }

  #[test]
  fn a_scene_that_never_completes_is_refused() {
    assert_eq!(SceneContext::completing(Scene::new(vec![])).unwrap_err(), NeverCompletes);
    assert_eq!(SceneContext::completing(timed(0.5, Cycle::Forever)).unwrap_err(), NeverCompletes);
  }

  #[test]
  fn one_that_isnt_checked_may_have_no_length() {
    assert_eq!(SceneContext::new(timed(0.5, Cycle::Once)).length(), Some(Duration::from_millis(500)));
    assert_eq!(SceneContext::new(timed(0.5, Cycle::Forever)).length(), None);
  }

  #[test]
  fn over_sets_how_long_a_cycle_takes() {
    let context = SceneContext::completing(timed(2.0, Cycle::Times(3))).unwrap().over(Duration::from_secs(1));
    assert_eq!(context.length(), Duration::from_secs(3));
    let unchanged = SceneContext::completing(timed(2.0, Cycle::Once)).unwrap().over(Duration::ZERO);
    assert_eq!(unchanged.length(), Duration::from_secs(2));
  }

  #[test]
  fn one_that_completes_can_go_where_either_is_taken() {
    let context: SceneContext = SceneContext::completing(timed(0.5, Cycle::Once)).unwrap().param("y", 1.0).into();
    assert_eq!(context.length(), Some(Duration::from_millis(500)));
    assert!(context.params().get("y").is_some());
  }
}
