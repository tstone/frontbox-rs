use frontbox_core::prelude::*;
use frontbox_scene::{PlayableScene, ScenePlayer};

use crate::GamePhase;
use crate::ball_management::{
  BallEnteredPlungeLane, BallExitedPlungeLane, BallReenteredPlungeLane,
};

/// A plunge (shooter) lane that balls are ejected into, and launched from. Built with `BallManager::standard`.
#[derive(Clone)]
pub struct PlungeLaneConfig {
  pub(super) switch_name: &'static str,
  /// How long the lane switch must stay open before the ball is considered gone, as a weak plunge can roll back
  pub(super) re_enter_timeout: Duration,
  /// Plays while a ball is in the lane during a game
  pub(super) ball_present_scene: Option<PlayableScene>,
  /// How long ball management waits for an ejected ball to reach the lane, before trying to knock it loose and then
  /// giving up on it
  pub(super) arrival_timeout: Duration,
}

impl PlungeLaneConfig {
  pub(super) fn new(switch_name: &'static str) -> Self {
    Self {
      switch_name,
      re_enter_timeout: Duration::from_millis(1000),
      ball_present_scene: None,
      arrival_timeout: Duration::from_millis(5000),
    }
  }
}

pub struct PlungeLaneSystem {
  handle: SystemHandle,
  plunge_lane_switch_name: &'static str,
  re_enter_timeout: Duration,
  /// From the ball entering the lane until it is considered gone
  ball_present: bool,
  wait_cue_id: Option<u64>,
  ball_present_scene: Option<ScenePlayer>,
}

impl PlungeLaneSystem {
  pub fn new(config: &PlungeLaneConfig) -> Self {
    Self {
      handle: SystemHandle::default(),
      plunge_lane_switch_name: config.switch_name,
      re_enter_timeout: config.re_enter_timeout,
      ball_present: false,
      wait_cue_id: None,
      ball_present_scene: config.ball_present_scene.clone().map(ScenePlayer::new),
    }
  }

  pub fn switch_definition(name: &'static str) -> SwitchDefinitionBuilder {
    SwitchDefinitionBuilder::new(name)
      .debounce_open(Duration::from_millis(75))
      .debounce_close(Duration::from_millis(100))
  }

  /// True from the ball entering the lane until it is considered gone (including the re-enter timeout)
  pub fn is_ball_present(&self) -> bool {
    self.ball_present
  }

  /// True if a ball is sitting on the lane switch right now
  pub fn is_ball_resting(&self, ctx: &ServiceContext) -> bool {
    let ctx = &ctx.for_system(self.handle);
    ctx
      .switches
      .is_closed(self.plunge_lane_switch_name)
      .unwrap_or(false)
  }

  fn cancel_cue(&mut self, ctx: &SystemContext) {
    ctx.cues.cancel(self.wait_cue_id.take());
  }

  fn on_lane_switch_closed(&mut self, ctx: &SystemContext) {
    // If a pending wait cue is running, then the ball has rolled back in rather than a new ball entering
    if self.wait_cue_id.is_some() {
      self.cancel_cue(ctx);
      log::info!(target: "frontbox::ball_management::plunge_lane", "Ball re-entered");
      ctx.emit(BallReenteredPlungeLane);
      return;
    }

    self.ball_present = true;
    log::info!(target: "frontbox::ball_management::plunge_lane", "Ball entered");
    ctx.emit(BallEnteredPlungeLane);
  }

  fn on_lane_switch_opened(&mut self, ctx: &SystemContext) {
    log::info!(target: "frontbox::ball_management::plunge_lane", "Ball exited");
    self.cancel_cue(ctx);
    self.wait_cue_id = Some(ctx.cues.start(TimesUpBallsGone, Cue::Once(self.re_enter_timeout)));
  }

  /// Plays the ball present scene while a ball is in the lane during a game, and stops it otherwise (e.g. a game quit
  /// with the ball still in the lane)
  fn update_ball_present_scene(&mut self, ctx: &SystemContext) {
    let show = self.ball_present && ctx.states.is(GamePhase::Active);
    if let Some(scene) = self.ball_present_scene.as_mut() {
      if show {
        scene.start(ctx);
      } else {
        scene.stop(ctx);
      }
    }
  }
}

impl System for PlungeLaneSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = ctx.handle();

    if ctx
      .switches
      .is_closed(self.plunge_lane_switch_name)
      .unwrap_or(false)
    {
      self.ball_present = true;
      ctx.emit(BallEnteredPlungeLane);
    }
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<TimesUpBallsGone>() {
      self.wait_cue_id = None;
      ctx.emit(BallExitedPlungeLane);
      self.ball_present = false;
    } else if !self.ball_present
      && let Some(event) = event.downcast_ref::<SwitchClosed>()
      && event.switch.name.eq(self.plunge_lane_switch_name)
    {
      self.on_lane_switch_closed(ctx);
    } else if self.ball_present
      && let Some(event) = event.downcast_ref::<SwitchOpened>()
      && event.switch.name.eq(self.plunge_lane_switch_name)
    {
      self.on_lane_switch_opened(ctx);
    } else if self.wait_cue_id.is_some()
      && let Some(event) = event.downcast_ref::<SwitchClosed>()
      && event.switch.name.eq(self.plunge_lane_switch_name)
    {
      self.on_lane_switch_closed(ctx);
    }
  }

  fn on_tick(&mut self, _delta: Duration, ctx: &SystemContext) {
    self.update_ball_present_scene(ctx);
  }
}

// -- Cues --

#[derive(serde::Serialize, Event)]
struct TimesUpBallsGone;

#[cfg(test)]
mod tests {
  use super::*;

  const PLUNGE: &str = "plunge";

  struct Fixture {
    lane: PlungeLaneSystem,
    ctx: TestContext,
    /// Sets the game phase, once one is set
    game_owner: Option<SystemHandle>,
  }

  struct GameOwner;
  impl System for GameOwner {}

  /// A scene on an LED the machine doesn't have, that plays until stopped
  fn scene() -> PlayableScene {
    let scene = frontbox_scene::Scene::load(
      r#"Scene(
        version: 2,
        layers: [Leds(leds: Name("plunge_led"), surface: Sequence, layer: (fill: (base: Solid(Fixed(255, 0, 0, 255)))))],
      )"#,
    );
    PlayableScene::new(scene.unwrap())
  }

  impl Fixture {
    fn spawn() -> Self {
      Self::spawn_with(PlungeLaneConfig::new(PLUNGE))
    }

    fn spawn_with(config: PlungeLaneConfig) -> Self {
      let mut ctx = TestContext::default();
      ctx.insert_switch(Switch {
        name: PLUNGE,
        id: 0,
        ..Default::default()
      });
      ctx.update_switch_states(vec![SwitchState::Open]);

      let mut lane = PlungeLaneSystem::new(&config);
      lane.on_spawn(&ctx.sys_ctx());
      Self {
        lane,
        ctx,
        game_owner: None,
      }
    }

    fn set_lane_switch(&mut self, closed: bool) {
      let state = if closed {
        SwitchState::Closed
      } else {
        SwitchState::Open
      };
      self.ctx.update_switch_states(vec![state]);
      let switch = self.ctx.base.switches.by_name(PLUNGE).unwrap().clone();
      if closed {
        self
          .lane
          .on_event(&SwitchClosed::new(switch), &self.ctx.sys_ctx());
      } else {
        self
          .lane
          .on_event(&SwitchOpened::new(switch), &self.ctx.sys_ctx());
      }
    }

    fn set_game_phase(&mut self, phase: GamePhase) {
      let owner = *self.game_owner.get_or_insert_with(|| self.ctx.insert_system(GameOwner));
      self.ctx.svc_ctx().for_system(owner).states.set(phase);
      self.ctx.apply_states();
    }

    fn scene_playing(&mut self) -> bool {
      self.lane.on_tick(Duration::from_millis(10), &self.ctx.sys_ctx());
      self.lane.ball_present_scene.as_ref().unwrap().is_started()
    }

    fn timeout_elapsed(&mut self) {
      self.lane.on_event(&TimesUpBallsGone, &self.ctx.sys_ctx());
    }

    fn events(&mut self) -> Vec<String> {
      self
        .ctx
        .events_emitted()
        .iter()
        .map(|e| e.short_name())
        .collect()
    }
  }

  #[test]
  fn ball_entering_reports_entered() {
    let mut f = Fixture::spawn();
    f.set_lane_switch(true);
    assert_eq!(f.events(), vec!["BallEnteredPlungeLane"]);
    assert!(f.lane.is_ball_present());
  }

  #[test]
  fn ball_is_gone_only_after_re_enter_timeout() {
    let mut f = Fixture::spawn();
    f.set_lane_switch(true);
    f.events();

    f.set_lane_switch(false);
    assert!(f.events().is_empty());
    assert!(f.lane.is_ball_present());

    f.timeout_elapsed();
    assert_eq!(f.events(), vec!["BallExitedPlungeLane"]);
    assert!(!f.lane.is_ball_present());
  }

  #[test]
  fn ball_rolling_back_before_timeout_reports_reentry_and_is_not_gone() {
    let mut f = Fixture::spawn();
    f.set_lane_switch(true);
    f.set_lane_switch(false);
    f.events();

    f.set_lane_switch(true);
    assert_eq!(f.events(), vec!["BallReenteredPlungeLane"]);
    assert!(f.lane.wait_cue_id.is_none(), "timeout cancelled");
    assert!(f.lane.is_ball_present());
  }

  #[test]
  fn ball_present_scene_only_plays_during_a_game() {
    let mut config = PlungeLaneConfig::new(PLUNGE);
    config.ball_present_scene = Some(scene());
    let mut f = Fixture::spawn_with(config);

    f.set_lane_switch(true);
    assert!(!f.scene_playing(), "no game");

    f.set_game_phase(GamePhase::Active);
    assert!(f.scene_playing());

    // quit with the ball still in the lane
    f.set_game_phase(GamePhase::Ending);
    assert!(!f.scene_playing());
  }

  #[test]
  fn ball_present_scene_stops_once_the_ball_is_gone() {
    let mut config = PlungeLaneConfig::new(PLUNGE);
    config.ball_present_scene = Some(scene());
    let mut f = Fixture::spawn_with(config);
    f.set_game_phase(GamePhase::Active);

    f.set_lane_switch(true);
    assert!(f.scene_playing());

    f.set_lane_switch(false);
    f.timeout_elapsed();
    assert!(!f.scene_playing());
  }

  #[test]
  fn is_ball_resting_reads_the_lane_switch() {
    let mut f = Fixture::spawn();
    f.set_lane_switch(true);
    assert!(f.lane.is_ball_resting(&f.ctx.svc_ctx()));
    f.set_lane_switch(false);
    assert!(!f.lane.is_ball_resting(&f.ctx.svc_ctx()));
  }
}
