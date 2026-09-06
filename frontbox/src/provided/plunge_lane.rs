use crate::{
  prelude::*,
  provided::{BallExitedTrough, PlungeLaneState::*},
};

pub struct PlungeLaneSystem {
  plunge_lane_switch_name: &'static str,
  expect_ball: bool,
  re_enter_timeout: Duration,
  state: PlungeLaneState,
  wait_cue_id: Option<u64>,
  ball_present_program: Option<LedProgram1d>,
}

impl PlungeLaneSystem {
  pub fn new(plunge_lane_switch_name: &'static str, re_enter_timeout: Duration) -> Self {
    Self {
      plunge_lane_switch_name,
      expect_ball: false,
      re_enter_timeout,
      state: PlungeLaneState::NoBall,
      wait_cue_id: None,
      ball_present_program: None,
    }
  }

  /// Add an effect when the ball is present in the plunge lane
  pub fn ball_present_effect(mut self, effect: LedProgram1d) -> Self {
    self.ball_present_program = Some(effect);
    self
  }

  pub fn switch_definition(name: &'static str) -> SwitchDefinitionBuilder {
    SwitchDefinitionBuilder::new(name)
      .debounce_open(Duration::from_millis(75))
      .debounce_close(Duration::from_millis(100))
  }

  pub fn expect_ball(&mut self) {
    self.expect_ball = true;
  }

  pub fn current_state(&self) -> &PlungeLaneState {
    &self.state
  }

  pub fn is_ball_present(&self) -> bool {
    self.state != PlungeLaneState::NoBall
  }

  fn cancel_cue(&mut self, ctx: &SystemContext) {
    if let Some(cue_id) = self.wait_cue_id {
      ctx.cancel_cue(cue_id);
      self.wait_cue_id = None;
    }
  }

  fn on_lane_switch_closed(&mut self, ctx: &SystemContext) {
    // If a pending wait cue is running, then the ball has re-entered
    // Cancel it and don't emit any events.
    if self.wait_cue_id.is_some() {
      self.cancel_cue(ctx);
      return;
    }

    if self.expect_ball {
      self.state = PlungeLaneState::ExpectedBallPresent;
    } else {
      self.state = PlungeLaneState::UnexpectedBallPresent;
    }

    log::info!("PlungeLane: Ball entered ({:?})", self.state);
    ctx.emit(BallEnteredPlungeLane::new(self.state));
    self.expect_ball = false;

    if let Some(effect) = self.ball_present_program.as_mut() {
      effect.play();
    }
  }

  fn on_lane_switch_opened(&mut self, ctx: &SystemContext) {
    log::info!("PlungeLane: Ball exited ({:?})", self.state);
    self.cancel_cue(ctx);
    self.wait_cue_id = Some(ctx.cue(TimesUpBallsGone, Cue::Once(self.re_enter_timeout)));
  }
}

impl System for PlungeLaneSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    // eject if ball is present
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
    if event.is::<BallExitedTrough>() {
      self.expect_ball = true;
    } else if event.is::<TimesUpBallsGone>() {
      self.wait_cue_id = None;
      ctx.emit(BallExitedPlungeLane);
      self.state = PlungeLaneState::NoBall;

      if let Some(effect) = self.ball_present_program.as_mut() {
        effect.stop(ctx);
      }
    } else if self.state == NoBall
      && let Some(event) = event.downcast_ref::<SwitchClosed>()
      && event.switch.name.eq(self.plunge_lane_switch_name)
    {
      self.on_lane_switch_closed(ctx);
    } else if (self.state == ExpectedBallPresent || self.state == UnexpectedBallPresent)
      && let Some(event) = event.downcast_ref::<SwitchOpened>()
      && event.switch.name.eq(self.plunge_lane_switch_name)
    {
      self.on_lane_switch_opened(ctx);
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

// Cues
#[derive(serde::Serialize, Event)]
struct TimesUpBallsGone;

// Public events
#[derive(serde::Serialize, Event)]
pub struct BallEnteredPlungeLane {
  pub state: PlungeLaneState,
}

impl BallEnteredPlungeLane {
  pub fn new(state: PlungeLaneState) -> Self {
    Self { state }
  }
}

#[derive(serde::Serialize, Event)]
pub struct BallExitedPlungeLane;
#[derive(serde::Serialize, Event)]
pub struct BallSaved; // TODO: set state as unexpected if ball was saved
