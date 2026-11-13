use frontbox_core::prelude::*;
use frontbox_scene::{Scene, ScenePlayer};

use crate::GamePhase;

/// Attract mode: while no game is running, plays a list of scenes, each until it completes, and starts over at the end of
/// the list
///
/// Each plays on whichever LEDs and plane it says. A scene that never completes (static, or repeating forever) plays until
/// the system goes inactive, and the scenes after it aren't reached. A scene that draws images needs them loaded with
/// [`Scene::with_images`].
///
/// ```rust,ignore
/// app.system(AttractSystem::new(vec![Scene::load(SWEEP)?, Scene::load(RAINBOW)?]));
/// ```
pub struct AttractSystem {
  scenes: Vec<ScenePlayer>,
  /// The scene that's playing, or that plays next if none is
  current: usize,
}

impl AttractSystem {
  pub fn new(scenes: Vec<Scene>) -> Self {
    Self { scenes: scenes.into_iter().map(ScenePlayer::new).collect(), current: 0 }
  }
}

impl System for AttractSystem {
  fn is_active(&self, ctx: &SystemContext) -> bool {
    ctx.states.is(GamePhase::None)
  }

  /// Picks up on the scene it left, from the top, rather than partway through
  fn on_reactivate(&mut self, ctx: &SystemContext) {
    if let Some(scene) = self.scenes.get_mut(self.current) {
      scene.stop(ctx);
    }
  }

  fn on_tick(&mut self, _delta: Duration, ctx: &SystemContext) {
    if self.scenes.is_empty() {
      return;
    }
    if self.scenes[self.current].is_complete(ctx) {
      self.scenes[self.current].stop(ctx);
      self.current = (self.current + 1) % self.scenes.len();
    }
    self.scenes[self.current].start(ctx);
  }
}

#[cfg(test)]
mod tests {
  use frontbox_gfx::animation::Cycle;
  use frontbox_gfx::prelude::*;
  use frontbox_gfx::scene::*;
  use frontbox_scene::{LedLayer, SceneSystem};

  use super::*;

  /// Stands in for the game manager, which owns `GamePhase`
  struct GameOwner;
  impl System for GameOwner {}

  fn context() -> TestContext {
    let mut context = TestContext::default();
    context.insert_system(SceneSystem::new());
    context
  }

  /// A scene that says what it is in its layer's name
  fn named(name: &str) -> Scene {
    let layer = Layer::new(Fill::new(FillNode::Solid(Color(Rgba::red()).into()))).name(name);
    Scene::new(vec![LedLayer::sequence(LedQ::every(), layer).into()])
  }

  /// One second of a scene
  fn second_of(name: &str) -> Scene {
    named(name).timing(Timing::new(1.0, Cycle::Once))
  }

  /// The names of the scenes showing
  fn showing(context: &TestContext) -> Vec<String> {
    let names = context.sys_ctx().systems.with::<SceneSystem, _>(|scenes| {
      scenes.shown().map(|(_, playback)| playback.scene().layers[0].layer().name.clone().unwrap()).collect()
    });
    names.unwrap()
  }

  /// Ticks the scenes, then attract mode, as the run loop would
  fn tick(attract: &mut AttractSystem, context: &TestContext, delta: Duration) {
    context.sys_ctx().systems.with::<SceneSystem, _>(|scenes| scenes.on_tick(delta, &context.sys_ctx()));
    attract.on_tick(delta, &context.sys_ctx());
  }

  /// Plays out the current scene
  fn second(attract: &mut AttractSystem, context: &TestContext) {
    tick(attract, context, Duration::from_secs(1));
  }

  #[test]
  fn plays_a_scene_until_it_completes() {
    let context = context();
    let mut attract = AttractSystem::new(vec![second_of("red"), second_of("blue")]);

    tick(&mut attract, &context, Duration::ZERO);
    tick(&mut attract, &context, Duration::from_millis(500));
    assert_eq!(showing(&context), ["red"]);
  }

  #[test]
  fn moves_to_the_next_scene_when_one_completes() {
    let context = context();
    let mut attract = AttractSystem::new(vec![second_of("red"), second_of("blue")]);

    tick(&mut attract, &context, Duration::ZERO);
    second(&mut attract, &context);
    assert_eq!(showing(&context), ["blue"]);
  }

  #[test]
  fn starts_over_at_the_end_of_the_list() {
    let context = context();
    let mut attract = AttractSystem::new(vec![second_of("red"), second_of("blue")]);

    tick(&mut attract, &context, Duration::ZERO);
    second(&mut attract, &context);
    second(&mut attract, &context);
    assert_eq!(showing(&context), ["red"]);
  }

  #[test]
  fn a_single_scene_repeats() {
    let context = context();
    let mut attract = AttractSystem::new(vec![second_of("red")]);

    tick(&mut attract, &context, Duration::ZERO);
    second(&mut attract, &context);
    assert_eq!(showing(&context), ["red"]);
  }

  #[test]
  fn a_scene_that_never_completes_holds_the_list_there() {
    let context = context();
    let mut attract = AttractSystem::new(vec![named("red"), second_of("blue")]);

    tick(&mut attract, &context, Duration::ZERO);
    tick(&mut attract, &context, Duration::from_secs(100));
    assert_eq!(showing(&context), ["red"]);
  }

  #[test]
  fn returns_to_the_top_of_the_scene_it_left() {
    let context = context();
    let mut attract = AttractSystem::new(vec![second_of("red"), second_of("blue")]);

    tick(&mut attract, &context, Duration::ZERO);
    second(&mut attract, &context);
    tick(&mut attract, &context, Duration::from_millis(500));
    attract.on_reactivate(&context.sys_ctx());

    tick(&mut attract, &context, Duration::from_millis(600));
    assert_eq!(showing(&context), ["blue"], "still the second scene, with a whole second to play");
  }

  #[test]
  fn only_runs_while_no_game_is_running() {
    let mut context = context();
    let owner = context.insert_system(GameOwner);
    let system = AttractSystem::new(vec![]);
    assert!(system.is_active(&context.svc_ctx().for_system(owner)));

    for phase in [GamePhase::Starting, GamePhase::Active, GamePhase::Ending] {
      context.svc_ctx().for_system(owner).states.set(phase);
      context.apply_states();
      assert!(!system.is_active(&context.svc_ctx().for_system(owner)), "{phase:?}");
    }

    context.svc_ctx().for_system(owner).states.set(GamePhase::None);
    context.apply_states();
    assert!(system.is_active(&context.svc_ctx().for_system(owner)));
  }

  #[test]
  fn no_scenes_does_nothing() {
    let context = context();
    let mut attract = AttractSystem::new(vec![]);

    attract.on_reactivate(&context.sys_ctx());
    tick(&mut attract, &context, Duration::from_millis(16));
    assert!(showing(&context).is_empty());
  }
}
