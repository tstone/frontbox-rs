use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use frontbox_core::prelude::*;
use frontbox_gfx::scene::{EndBehavior, Params, Value};

use crate::*;

/// A [`Scene`] with values from the game bound to it: its parameters, and how long it takes. It holds nothing about a
/// scene that's playing, so one can be kept and played any number of times. A `Scene` converts into one that plays as
/// written.
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
/// let scene = PlayableScene::new(scene).param("color", Rgba::red());
/// ```
///
/// What needs a scene to go away on its own, such as [`play_scene`](SceneSystemExt::play_scene), takes a
/// `PlayableScene<Completes>`, which only [`PlayableScene::completing`] makes:
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
/// let scene = PlayableScene::completing(scene).unwrap();
/// assert_eq!(scene.length(), Duration::from_millis(500));
/// ```
#[derive(Debug, Clone)]
pub struct PlayableScene<C = Unchecked> {
  scene: Arc<Scene>,
  params: Params,
  /// How much of the scene plays in a second
  speed: f32,
  completes: PhantomData<C>,
}

impl PlayableScene {
  pub fn new(scene: impl Into<Arc<Scene>>) -> Self {
    Self::unchecked(scene.into())
  }

  /// How long the scene takes to play through, at the speed it's played at. `None` for a scene that never completes:
  /// one that's static or repeats forever.
  pub fn length(&self) -> Option<Duration> {
    Some(Duration::from_secs_f32(self.scene_length()? / self.speed))
  }
}

impl PlayableScene<Completes> {
  /// For a scene that plays through and goes away. One that's static, repeats forever, or holds its last moment is
  /// refused.
  pub fn completing(scene: impl Into<Arc<Scene>>) -> Result<Self, NeverCompletes> {
    let playable = PlayableScene::unchecked(scene.into());
    let clears = playable.scene.timing.is_some_and(|timing| timing.end == EndBehavior::Clear);
    match playable.scene_length() {
      Some(_) if clears => Ok(playable),
      _ => Err(NeverCompletes),
    }
  }

  /// How long the scene takes to play through, at the speed it's played at
  pub fn length(&self) -> Duration {
    // checked to have a length by `completing`
    Duration::from_secs_f32(self.scene_length().unwrap_or_default() / self.speed)
  }
}

impl<C> PlayableScene<C> {
  fn unchecked(scene: Arc<Scene>) -> Self {
    Self { scene, params: Params::new(), speed: 1.0, completes: PhantomData }
  }

  /// Sets a parameter, taking the place of the scene's default
  pub fn param(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
    self.set_param(name, value);
    self
  }

  /// [`PlayableScene::param`] for one that's already built
  pub fn set_param(&mut self, name: impl Into<String>, value: impl Into<Value>) {
    self.params.insert(name.into(), value.into());
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

  pub(crate) fn speed(&self) -> f32 {
    self.speed
  }

  /// Seconds for every cycle to play as written, or `None` if that never happens
  fn scene_length(&self) -> Option<f32> {
    self.scene.timing?.length()
  }
}

impl From<Scene> for PlayableScene {
  fn from(scene: Scene) -> Self {
    Self::new(scene)
  }
}

impl From<Arc<Scene>> for PlayableScene {
  fn from(scene: Arc<Scene>) -> Self {
    Self::new(scene)
  }
}

impl From<PlayableScene<Completes>> for PlayableScene {
  fn from(playable: PlayableScene<Completes>) -> Self {
    Self { scene: playable.scene, params: playable.params, speed: playable.speed, completes: PhantomData }
  }
}

/// Marks a [`PlayableScene`] whose scene may or may not go away on its own
#[derive(Debug, Clone, Copy)]
pub struct Unchecked;

/// Marks a [`PlayableScene`] whose scene plays through and goes away
#[derive(Debug, Clone, Copy)]
pub struct Completes;

/// Why [`PlayableScene::completing`] refused: the scene is static, repeats forever, or holds its last moment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NeverCompletes;

impl fmt::Display for NeverCompletes {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "the scene never goes away. Give it a timing that plays a set number of times and clears at the end")
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
    let playable = PlayableScene::completing(timed(0.5, Cycle::Times(2))).unwrap();
    assert_eq!(playable.length(), Duration::from_secs(1));
  }

  #[test]
  fn a_scene_that_never_completes_is_refused() {
    assert_eq!(PlayableScene::completing(Scene::new(vec![])).unwrap_err(), NeverCompletes);
    assert_eq!(PlayableScene::completing(timed(0.5, Cycle::Forever)).unwrap_err(), NeverCompletes);
  }

  #[test]
  fn a_scene_that_holds_its_end_is_refused() {
    let held = Scene::new(vec![]).timing(Timing::new(0.5, Cycle::Once).end(EndBehavior::Hold));
    assert_eq!(PlayableScene::completing(held).unwrap_err(), NeverCompletes);
  }

  #[test]
  fn one_that_isnt_checked_may_have_no_length() {
    assert_eq!(PlayableScene::new(timed(0.5, Cycle::Once)).length(), Some(Duration::from_millis(500)));
    assert_eq!(PlayableScene::new(timed(0.5, Cycle::Forever)).length(), None);
  }

  #[test]
  fn over_sets_how_long_a_cycle_takes() {
    let playable = PlayableScene::completing(timed(2.0, Cycle::Times(3))).unwrap().over(Duration::from_secs(1));
    assert_eq!(playable.length(), Duration::from_secs(3));
    let unchanged = PlayableScene::completing(timed(2.0, Cycle::Once)).unwrap().over(Duration::ZERO);
    assert_eq!(unchanged.length(), Duration::from_secs(2));
  }

  #[test]
  fn one_that_completes_can_go_where_either_is_taken() {
    let playable: PlayableScene = PlayableScene::completing(timed(0.5, Cycle::Once)).unwrap().param("y", 1.0).into();
    assert_eq!(playable.length(), Some(Duration::from_millis(500)));
    assert!(playable.params().get("y").is_some());
  }
}
