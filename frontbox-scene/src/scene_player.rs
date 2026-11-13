use frontbox_core::prelude::*;
use frontbox_gfx::animation::Curve;
use frontbox_gfx::scene::Value;

use crate::*;

/// Start and stop playback of a scene, played by the [`SceneSystem`] and owned by the system that starts it. It's shown
/// while that system is active, and goes away when the system despawns or goes inactive.
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # use frontbox_scene::*;
/// pub struct ShotLit {
///   lit: bool,
///   scene: ScenePlayer,
/// }
///
/// impl System for ShotLit {
///   fn on_tick(&mut self, _delta: Duration, ctx: &SystemContext) {
///     if self.lit {
///       self.scene.start(ctx);
///     } else {
///       self.scene.stop(ctx);
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ScenePlayer<C = Unchecked> {
  scene: PlayableScene<C>,
  playing: Option<PlaybackId>,
}

impl<C> ScenePlayer<C> {
  pub fn new(scene: impl Into<PlayableScene<C>>) -> Self {
    Self {
      scene: scene.into(),
      playing: None,
    }
  }

  /// [`ScenePlayer::new`], started
  pub fn started(scene: impl Into<PlayableScene<C>>, ctx: &SystemContext) -> Self {
    let mut player = Self::new(scene);
    player.start(ctx);
    player
  }

  /// Plays from the top, unless it's already started. Safe to call every tick.
  pub fn start(&mut self, ctx: &SystemContext) {
    if self.is_started() {
      return;
    }
    let id = match ctx.systems.get::<SceneSystem>() {
      Some(mut scenes) => scenes.start(ctx.id(), &self.scene),
      None => {
        log::error!(target: "frontbox::scene", "SceneSystem not running.");
        PlaybackId::none()
      }
    };
    self.playing = Some(id);
  }

  /// Plays from the top, whether or not it's started
  pub fn restart(&mut self, ctx: &SystemContext) {
    self.stop(ctx);
    self.start(ctx);
  }

  /// Takes it off, unless it's already stopped. Safe to call every tick.
  pub fn stop(&mut self, ctx: &SystemContext) {
    if let Some(id) = self.playing.take() {
      ctx
        .systems
        .with::<SceneSystem, _>(|scenes| scenes.remove(id));
    }
  }

  /// Plays `scene` in place of the one it has: from the top now if it's started, otherwise once it's started
  pub fn replace_scene(&mut self, scene: impl Into<PlayableScene<C>>, ctx: &SystemContext) {
    self.scene = scene.into();
    if self.is_started() {
      self.restart(ctx);
    }
  }

  /// Whether it was started and hasn't been stopped since, even if it has completed
  pub fn is_started(&self) -> bool {
    self.playing.is_some()
  }

  /// Whether it was started and has played through. A scene that's static or repeats forever never does.
  pub fn is_complete(&self, ctx: &SystemContext) -> bool {
    let Some(id) = self.playing else {
      return false;
    };
    // one that's gone has completed, as stopping it would have cleared `playing`
    let complete = ctx.systems.with::<SceneSystem, _>(|scenes| {
      scenes
        .playback(id)
        .is_none_or(|playback| playback.is_complete())
    });
    complete.unwrap_or(false)
  }

  /// Sets a parameter, now if it's playing, and for each time it starts from here on
  pub fn set_param(
    &mut self,
    name: impl Into<String>,
    value: impl Into<Value>,
    ctx: &SystemContext,
  ) {
    let (name, value) = (name.into(), value.into());
    self.with_playback(ctx, |playback| playback.set_param(name.clone(), value));
    self.scene.set_param(name, value);
  }

  /// Moves a parameter to `to` over `duration`, along `curve`, if it's playing: see [`Playback::ease_param`]. Each time
  /// it starts from here on, the parameter is `to` from the top.
  pub fn ease_param(
    &mut self,
    name: impl Into<String>,
    to: impl Into<Value>,
    duration: Duration,
    curve: Curve,
    ctx: &SystemContext,
  ) {
    let (name, to) = (name.into(), to.into());
    self.with_playback(ctx, |playback| {
      playback.ease_param(name.clone(), to, duration, curve)
    });
    self.scene.set_param(name, to);
  }

  /// The scene and the values bound to it, such as its [`length`](PlayableScene::length)
  pub fn scene(&self) -> &PlayableScene<C> {
    &self.scene
  }

  fn with_playback(&self, ctx: &SystemContext, f: impl FnOnce(&mut Playback)) {
    if let Some(id) = self.playing {
      ctx
        .systems
        .with::<SceneSystem, _>(|scenes| scenes.playback(id).map(f));
    }
  }
}

#[cfg(test)]
mod tests {
  use frontbox_gfx::animation::Cycle;
  use frontbox_gfx::scene::{EndBehavior, Timing};

  use super::*;

  fn timed(duration: f32, cycle: Cycle) -> Scene {
    Scene::new(vec![]).timing(Timing::new(duration, cycle))
  }

  fn context() -> TestContext {
    let mut context = TestContext::default();
    context.insert_system(SceneSystem::new());
    context
  }

  fn tick(context: &TestContext, delta: Duration) {
    context
      .sys_ctx()
      .systems
      .with::<SceneSystem, _>(|scenes| scenes.on_tick(delta, &context.sys_ctx()));
  }

  /// The time of each playback shown
  fn times(context: &TestContext) -> Vec<f32> {
    context
      .sys_ctx()
      .systems
      .with::<SceneSystem, _>(|scenes| {
        scenes
          .shown()
          .map(|(_, playback)| playback.moment().time)
          .collect()
      })
      .unwrap()
  }

  fn float(context: &TestContext, name: &str) -> Option<f32> {
    let value = context.sys_ctx().systems.with::<SceneSystem, _>(|scenes| {
      scenes
        .shown()
        .next()
        .and_then(|(_, playback)| playback.moment().params.get(name).copied())
    });
    match value.unwrap() {
      Some(Value::Float(value)) => Some(value),
      _ => None,
    }
  }

  #[test]
  fn it_plays_nothing_until_started() {
    let context = context();
    let mut player: ScenePlayer = ScenePlayer::new(timed(1.0, Cycle::Forever));
    assert!(!player.is_started() && times(&context).is_empty());

    player.start(&context.sys_ctx());
    assert!(player.is_started());
    assert_eq!(times(&context), [0.0]);
  }

  #[test]
  fn restarting_plays_from_the_top() {
    let context = context();
    let mut player: ScenePlayer = ScenePlayer::new(timed(1.0, Cycle::Forever));
    player.start(&context.sys_ctx());
    tick(&context, Duration::from_millis(500));
    assert_eq!(times(&context), [0.5]);

    player.restart(&context.sys_ctx());
    assert_eq!(times(&context), [0.0]);
  }

  #[test]
  fn stopping_removes_it() {
    let context = context();
    let mut player: ScenePlayer = ScenePlayer::new(timed(1.0, Cycle::Forever));
    player.start(&context.sys_ctx());
    player.stop(&context.sys_ctx());
    assert!(!player.is_started() && times(&context).is_empty());
  }

  #[test]
  fn starting_one_thats_started_leaves_it_playing() {
    let context = context();
    let mut player: ScenePlayer = ScenePlayer::new(timed(1.0, Cycle::Forever));
    player.start(&context.sys_ctx());
    tick(&context, Duration::from_millis(500));
    player.start(&context.sys_ctx());
    assert_eq!(times(&context), [0.5]);

    player.stop(&context.sys_ctx());
    player.stop(&context.sys_ctx());
    assert!(times(&context).is_empty());
  }

  #[test]
  fn a_new_scene_plays_from_the_top_in_place_of_the_old_one() {
    let context = context();
    let mut player: ScenePlayer = ScenePlayer::new(timed(1.0, Cycle::Forever));
    player.replace_scene(timed(2.0, Cycle::Forever), &context.sys_ctx());
    assert!(times(&context).is_empty(), "not started");

    player.start(&context.sys_ctx());
    tick(&context, Duration::from_millis(500));
    player.replace_scene(timed(1.0, Cycle::Forever), &context.sys_ctx());
    assert_eq!(times(&context), [0.0]);
  }

  #[test]
  fn it_completes_once_the_scene_has_played_through() {
    let context = context();
    let mut player: ScenePlayer = ScenePlayer::new(timed(1.0, Cycle::Once));
    assert!(!player.is_complete(&context.sys_ctx()), "not started");
    player.start(&context.sys_ctx());
    tick(&context, Duration::from_millis(500));
    assert!(!player.is_complete(&context.sys_ctx()));
    tick(&context, Duration::from_millis(600));
    assert!(player.is_complete(&context.sys_ctx()) && player.is_started());
  }

  #[test]
  fn one_that_holds_completes_while_still_shown() {
    let context = context();
    let mut player = ScenePlayer::new(
      Scene::new(vec![]).timing(Timing::new(1.0, Cycle::Once).end(EndBehavior::Hold)),
    );
    player.start(&context.sys_ctx());
    tick(&context, Duration::from_secs(2));
    assert!(player.is_complete(&context.sys_ctx()));
    assert_eq!(times(&context).len(), 1);
  }

  #[test]
  fn a_param_set_while_playing_is_kept_for_the_next_start() {
    let context = context();
    let mut player: ScenePlayer = ScenePlayer::new(timed(1.0, Cycle::Forever));
    player.start(&context.sys_ctx());
    player.set_param("y", 2.0, &context.sys_ctx());
    assert_eq!(float(&context, "y"), Some(2.0));

    player.start(&context.sys_ctx());
    assert_eq!(float(&context, "y"), Some(2.0));
  }

  #[test]
  fn an_eased_param_moves_while_playing() {
    let context = context();
    let mut player =
      ScenePlayer::new(PlayableScene::new(timed(1.0, Cycle::Forever)).param("y", 0.0));
    player.start(&context.sys_ctx());
    player.ease_param(
      "y",
      10.0,
      Duration::from_secs(1),
      Curve::Linear,
      &context.sys_ctx(),
    );
    tick(&context, Duration::from_millis(500));
    assert_eq!(float(&context, "y"), Some(5.0));

    player.restart(&context.sys_ctx());
    assert_eq!(float(&context, "y"), Some(10.0));
  }

  #[test]
  fn without_a_scene_system_nothing_plays() {
    let context = TestContext::default();
    let mut player: ScenePlayer = ScenePlayer::new(timed(1.0, Cycle::Once));
    player.start(&context.sys_ctx());
    assert!(player.is_started() && !player.is_complete(&context.sys_ctx()));
  }
}
