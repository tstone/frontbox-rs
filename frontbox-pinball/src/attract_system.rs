use frontbox_core::prelude::*;
use frontbox_led::prelude::*;

use crate::GamePhase;

/// Attract mode: while no game is running, plays a list of scenes, each until it completes, and starts over at the end of
/// the list
///
/// Each plays on whichever LEDs and plane it says. A scene that never completes (static, or repeating forever) plays until
/// the system goes inactive, and the scenes after it aren't reached. Scenes play without images, so a layer that draws one
/// shows nothing.
///
/// ```rust,ignore
/// app.system(AttractSystem::new(vec![LedScene::load(SWEEP)?, LedScene::load(RAINBOW)?])?);
/// ```
pub struct AttractSystem {
  scenes: Vec<LedPlayback>,
  current: usize,
}

impl AttractSystem {
  /// A scene with a layer that isn't on any LEDs yet is refused
  pub fn new(scenes: Vec<LedScene>) -> Result<Self, PlaybackError> {
    let scenes = scenes.into_iter().map(LedPlayback::new).collect::<Result<_, _>>()?;
    Ok(Self { scenes, current: 0 })
  }

  /// Stop the current scene, whichever LEDs it was on, and play the one after it
  fn next_scene(&mut self, ctx: &SystemContext) {
    self.scenes[self.current].stop(ctx);
    self.current = (self.current + 1) % self.scenes.len();
    self.scenes[self.current].play();
  }
}

impl System for AttractSystem {
  fn is_active(&self, ctx: &SystemContext) -> bool {
    ctx.states.is(GamePhase::None)
  }

  /// Picks up on the scene it left, from the top, rather than partway through
  fn on_reactivate(&mut self, _ctx: &SystemContext) {
    if let Some(scene) = self.scenes.get_mut(self.current) {
      scene.reset();
    }
  }

  fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {
    if self.scenes.is_empty() {
      return;
    }
    if self.scenes[self.current].is_complete() {
      self.next_scene(ctx);
    }
    self.scenes[self.current].apply(delta, ctx);
  }
}

#[cfg(test)]
mod tests {

  use frontbox_gfx::animation::Cycle;
  use frontbox_gfx::prelude::*;
  use frontbox_gfx::scene::*;

  use super::*;

  /// Stands in for the game manager, which owns `GamePhase`
  struct GameOwner;
  impl System for GameOwner {}

  static PLANE: ReferencePlane = ReferencePlane::new("Plane").extent(Vec2::new(10.0, 10.0)).build();

  /// One LED on the plane, and one off it
  fn two_leds() -> (TestContext, Vec<LedAddress>) {
    let mut context = TestContext::default();
    context.insert_system(LedSystem::new());
    let addresses: Vec<_> = (1..=2).map(|i| LedAddress::new(ExpAddress::default(), i)).collect();
    for (i, x) in [5.0, 50.0].into_iter().enumerate() {
      context.insert_led(LED {
        name: format!("led{}", i + 1),
        address: addresses[i].clone(),
        location: Some(Vec3::new(x, 5.0, 0.0)),
        ..Default::default()
      });
    }
    context.add_plane(&PLANE);
    (context, addresses)
  }

  fn colors(context: &TestContext, addresses: &[LedAddress]) -> Vec<Option<Rgba<u8>>> {
    let ctx = context.sys_ctx();
    let led_system = ctx.systems.expect::<LedSystem>();
    addresses
      .iter()
      .map(|a| led_system.declared_colors(a).first().copied())
      .collect()
  }

  fn layer(color: Rgba<u8>) -> Layer {
    Layer::new(Fill::new(FillNode::Solid(Color(color).into())))
  }

  fn solid(color: Rgba<u8>) -> LedScene {
    LedScene::new(vec![LedLayer::sequence(LedQ::every(), layer(color))])
  }

  /// One second of `color`
  fn second_of(color: Rgba<u8>) -> LedScene {
    solid(color).timing(Timing::new(1.0, Cycle::Once))
  }

  /// One second of `color` across the plane
  fn second_across_plane(color: Rgba<u8>) -> LedScene {
    LedScene::new(vec![LedLayer::area(LedQ::every(), "Plane", layer(color))]).timing(Timing::new(1.0, Cycle::Once))
  }

  fn attract(scenes: Vec<LedScene>) -> AttractSystem {
    AttractSystem::new(scenes).unwrap()
  }

  fn tick(attract: &mut AttractSystem, context: &TestContext, delta: Duration) {
    attract.on_tick(delta, &context.sys_ctx());
  }

  /// Plays out the current scene, up to its last moment
  fn second(attract: &mut AttractSystem, context: &TestContext) {
    tick(attract, context, Duration::from_secs(1));
  }

  #[test]
  fn plays_a_scene_until_it_completes() {
    let (context, addresses) = two_leds();
    let mut attract = attract(vec![second_of(Rgba::red()), second_of(Rgba::blue())]);

    tick(&mut attract, &context, Duration::from_millis(500));
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));

    tick(&mut attract, &context, Duration::from_millis(500));
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));
  }

  #[test]
  fn moves_to_the_next_scene_when_one_completes() {
    let (context, addresses) = two_leds();
    let mut attract = attract(vec![second_of(Rgba::red()), second_of(Rgba::blue())]);

    second(&mut attract, &context);
    tick(&mut attract, &context, Duration::from_millis(16));
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::blue()));
  }

  #[test]
  fn a_scene_leaves_nothing_behind_on_leds_the_next_one_skips() {
    let (context, addresses) = two_leds();
    let mut attract = attract(vec![second_of(Rgba::red()), second_across_plane(Rgba::blue())]);

    second(&mut attract, &context);
    tick(&mut attract, &context, Duration::from_millis(16));
    assert_eq!(colors(&context, &addresses), vec![Some(Rgba::blue()), None]);
  }

  #[test]
  fn a_sequence_scene_plays_on_every_led() {
    let (context, addresses) = two_leds();
    let mut attract = attract(vec![second_of(Rgba::red())]);

    tick(&mut attract, &context, Duration::from_millis(16));
    assert_eq!(colors(&context, &addresses), vec![Some(Rgba::red()), Some(Rgba::red())]);
  }

  #[test]
  fn an_area_scene_plays_on_every_led_on_the_plane() {
    let (context, addresses) = two_leds();
    let mut attract = attract(vec![second_across_plane(Rgba::red())]);

    tick(&mut attract, &context, Duration::from_millis(16));
    assert_eq!(colors(&context, &addresses), vec![Some(Rgba::red()), None]);
  }

  #[test]
  fn starts_over_at_the_end_of_the_list() {
    let (context, addresses) = two_leds();
    let mut attract = attract(vec![second_of(Rgba::red()), second_of(Rgba::blue())]);

    second(&mut attract, &context);
    second(&mut attract, &context);
    tick(&mut attract, &context, Duration::from_millis(16));
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));
  }

  #[test]
  fn each_time_through_plays_the_scenes_in_full() {
    let (context, _) = two_leds();
    let mut attract = attract(vec![second_of(Rgba::red()), second_of(Rgba::blue())]);

    second(&mut attract, &context);
    second(&mut attract, &context);
    // back on the first scene, which has a whole second to play again
    tick(&mut attract, &context, Duration::from_millis(500));
    assert_eq!(attract.current, 0);
    assert!(!attract.scenes[0].is_complete());
  }

  #[test]
  fn a_single_scene_repeats() {
    let (context, addresses) = two_leds();
    let mut attract = attract(vec![second_of(Rgba::red())]);

    second(&mut attract, &context);
    tick(&mut attract, &context, Duration::from_millis(16));
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));
    assert!(!attract.scenes[0].is_complete());
  }

  #[test]
  fn a_scene_that_never_completes_holds_the_list_there() {
    let (context, addresses) = two_leds();
    let mut attract = attract(vec![solid(Rgba::red()), second_of(Rgba::blue())]);

    tick(&mut attract, &context, Duration::from_secs(100));
    tick(&mut attract, &context, Duration::from_millis(16));
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));
  }

  #[test]
  fn returns_to_the_top_of_the_scene_it_left() {
    let (context, _) = two_leds();
    let mut attract = attract(vec![second_of(Rgba::red()), second_of(Rgba::blue())]);

    second(&mut attract, &context);
    tick(&mut attract, &context, Duration::from_millis(500));
    attract.on_deactivate(&context.sys_ctx());
    attract.on_reactivate(&context.sys_ctx());

    tick(&mut attract, &context, Duration::from_millis(500));
    assert_eq!(attract.current, 1);
    assert!(!attract.scenes[1].is_complete());
  }

  #[test]
  fn only_runs_while_no_game_is_running() {
    let (mut context, _) = two_leds();
    let owner = context.insert_system(GameOwner);
    let system = attract(vec![]);
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
    let (context, addresses) = two_leds();
    let mut attract = attract(vec![]);

    attract.on_reactivate(&context.sys_ctx());
    tick(&mut attract, &context, Duration::from_millis(16));
    assert_eq!(colors(&context, &addresses), vec![None, None]);
  }
}
