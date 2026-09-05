use crate::{prelude::*, provided::BallExitedTrough};

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
}

impl System for PlungeLaneSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
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
    } else if let Some(event) = event.downcast_ref::<SwitchClosed>()
      && event.switch.name.eq(self.plunge_lane_switch_name)
    {
      // If a pending wait cue is running, then the ball has re-entered
      // Cancel it and don't emit any events.
      if let Some(cue_id) = self.wait_cue_id {
        ctx.cancel_cue(cue_id);
        self.wait_cue_id = None;
        return;
      }

      if self.expect_ball {
        self.state = PlungeLaneState::ExpectedBallPresent;
      } else {
        self.state = PlungeLaneState::UnexpectedBallPresent;
      }

      log::info!("Ball entered plunge lane ({:?})", self.state);
      ctx.emit(BallEnteredPlungeLane::new(self.state));
      self.expect_ball = false;

      if let Some(effect) = self.ball_present_program.as_mut() {
        effect.play();
      }
    } else if let Some(event) = event.downcast_ref::<SwitchOpened>()
      && event.switch.name.eq(self.plunge_lane_switch_name)
    {
      self.wait_cue_id = Some(ctx.cue(TimesUpBallsGone, Cue::Once(self.re_enter_timeout)));
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
