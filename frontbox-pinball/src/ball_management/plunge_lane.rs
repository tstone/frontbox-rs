use frontbox::prelude::*;

use crate::ball_management::{
  BallEnteredPlungeLane, BallExitedPlungeLane, BallReenteredPlungeLane,
};

/// A plunge (shooter) lane that balls are ejected into, and launched from. Built with `BallManager::standard`.
#[derive(Clone)]
pub struct PlungeLaneConfig {
  pub(super) switch_name: &'static str,
  /// How long the lane switch must stay open before the ball is considered gone, as a weak plunge can roll back
  pub(super) re_enter_timeout: Duration,
  pub(super) ball_present_effect: Option<LedProgram1d>,
}

impl PlungeLaneConfig {
  pub(super) fn new(switch_name: &'static str) -> Self {
    Self {
      switch_name,
      re_enter_timeout: Duration::from_millis(1000),
      ball_present_effect: None,
    }
  }
}

pub struct PlungeLaneSystem {
  handle: SystemHandle,
  plunge_lane_switch_name: &'static str,
  expect_ball: bool,
  re_enter_timeout: Duration,
  state: PlungeLaneState,
  wait_cue_id: Option<u64>,
  ball_present_program: Option<LedProgram1d>,
}

impl PlungeLaneSystem {
  pub fn new(config: &PlungeLaneConfig) -> Self {
    Self {
      handle: SystemHandle::default(),
      plunge_lane_switch_name: config.switch_name,
      expect_ball: false,
      re_enter_timeout: config.re_enter_timeout,
      state: PlungeLaneState::NoBall,
      wait_cue_id: None,
      ball_present_program: config.ball_present_effect.clone(),
    }
  }

  pub fn switch_definition(name: &'static str) -> SwitchDefinitionBuilder {
    SwitchDefinitionBuilder::new(name)
      .debounce_open(Duration::from_millis(75))
      .debounce_close(Duration::from_millis(100))
  }

  /// The next ball to enter the lane was ejected on purpose
  pub fn expect_ball(&mut self) {
    self.expect_ball = true;
  }

  pub fn current_state(&self) -> &PlungeLaneState {
    &self.state
  }

  /// True from the ball entering the lane until it is considered gone (including the re-enter timeout)
  pub fn is_ball_present(&self) -> bool {
    self.state != PlungeLaneState::NoBall
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
    if let Some(cue_id) = self.wait_cue_id {
      ctx.cancel_cue(cue_id);
      self.wait_cue_id = None;
    }
  }

  fn on_lane_switch_closed(&mut self, ctx: &SystemContext) {
    // If a pending wait cue is running, then the ball has rolled back in rather than a new ball entering
    if self.wait_cue_id.is_some() {
      self.cancel_cue(ctx);
      log::info!(target: "frontbox::plunge_lane", "Ball re-entered");
      ctx.emit(BallReenteredPlungeLane);
      return;
    }

    if self.expect_ball {
      self.state = PlungeLaneState::ExpectedBallPresent;
    } else {
      self.state = PlungeLaneState::UnexpectedBallPresent;
    }

    log::info!(target: "frontbox::plunge_lane", "Ball entered ({:?})", self.state);
    ctx.emit(BallEnteredPlungeLane::new(self.state));
    self.expect_ball = false;

    if let Some(effect) = self.ball_present_program.as_mut() {
      effect.play();
    }
  }

  fn on_lane_switch_opened(&mut self, ctx: &SystemContext) {
    log::info!(target: "frontbox::plunge_lane", "Ball exited ({:?})", self.state);
    self.cancel_cue(ctx);
    self.wait_cue_id = Some(ctx.cue(TimesUpBallsGone, Cue::Once(self.re_enter_timeout)));
  }
}

impl System for PlungeLaneSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = *ctx.current_handle();

    if ctx
      .switches
      .is_closed(self.plunge_lane_switch_name)
      .unwrap_or(false)
    {
      self.state = PlungeLaneState::UnexpectedBallPresent;
      ctx.emit(BallEnteredPlungeLane::new(self.state));
    }
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<TimesUpBallsGone>() {
      self.wait_cue_id = None;
      ctx.emit(BallExitedPlungeLane);
      self.state = PlungeLaneState::NoBall;

      if let Some(effect) = self.ball_present_program.as_mut() {
        effect.stop(ctx);
      }
    } else if self.state == PlungeLaneState::NoBall
      && let Some(event) = event.downcast_ref::<SwitchClosed>()
      && event.switch.name.eq(self.plunge_lane_switch_name)
    {
      self.on_lane_switch_closed(ctx);
    } else if (self.state == PlungeLaneState::ExpectedBallPresent
      || self.state == PlungeLaneState::UnexpectedBallPresent)
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

  fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {
    let ball_present = self.is_ball_present();
    if ball_present && let Some(effect) = self.ball_present_program.as_mut() {
      effect.apply(delta, ctx);
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum PlungeLaneState {
  NoBall,
  ExpectedBallPresent,
  UnexpectedBallPresent,
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
  }

  impl Fixture {
    fn spawn() -> Self {
      let mut ctx = TestContext::default();
      ctx.insert_switch(Switch {
        name: PLUNGE,
        id: 0,
        ..Default::default()
      });
      ctx.update_switch_states(vec![SwitchState::Open]);

      let mut lane = PlungeLaneSystem::new(&PlungeLaneConfig::new(PLUNGE));
      lane.on_spawn(&ctx.sys_ctx());
      Self { lane, ctx }
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
  fn expected_ball_entering_reports_expected() {
    let mut f = Fixture::spawn();
    f.lane.expect_ball();
    f.set_lane_switch(true);
    assert_eq!(f.events(), vec!["BallEnteredPlungeLane"]);
    assert_eq!(
      *f.lane.current_state(),
      PlungeLaneState::ExpectedBallPresent
    );
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
  fn is_ball_resting_reads_the_lane_switch() {
    let mut f = Fixture::spawn();
    f.set_lane_switch(true);
    assert!(f.lane.is_ball_resting(&f.ctx.svc_ctx()));
    f.set_lane_switch(false);
    assert!(!f.lane.is_ball_resting(&f.ctx.svc_ctx()));
  }
}
