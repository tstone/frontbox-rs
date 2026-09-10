use crate::prelude::*;
use crate::provided::autoplunger::action_button_eject::State::*;
use crate::provided::{AutoPlungerSystem, BallEnteredPlungeLane, BallExitedPlungeLane};

/// A system to fire the auto plunger when the action button is pressed
/// Button can only be pressed when there is a ball in the plunge lane
pub struct ActionButtonEject {
  action_button_switch: SwitchQ,
  effect: LedProgram1d,
  state: State,
}

impl ActionButtonEject {
  pub fn new(action_button_switch: SwitchQ, effect: LedProgram1d) -> Self {
    Self {
      action_button_switch,
      effect,
      state: BallNotInPlungeLane,
    }
  }

  fn ball_in_plunge_lane(&mut self) {
    self.effect.reset();
    self.effect.play();
    self.state = BallInPlungeLane;
  }

  fn ball_no_longer_in_plunge_lane(&mut self, ctx: &SystemContext) {
    self.effect.stop(ctx);
    self.state = BallNotInPlungeLane;
  }
}

impl System for ActionButtonEject {
  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<BallEnteredPlungeLane>() {
      self.ball_in_plunge_lane();
    } else if event.is::<BallExitedPlungeLane>() {
      self.ball_no_longer_in_plunge_lane(ctx);
    } else if let Some(e) = event.downcast_ref::<SwitchClosed>()
      && self.action_button_switch.matches(&e.switch)
      && self.state == BallInPlungeLane
    {
      if let Some(mut autoplunger) = ctx.get::<AutoPlungerSystem>() {
        log::info!(target: "frontbox::autoplunger", "Launching ball via action button press");
        autoplunger.fire(ctx.into());
      } else {
        log::warn!(target: "frontbox::autoplunger", "Action button pressed to launch but no AutoPlungerSystem present");
      }
    }
  }

  fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {
    self.effect.apply(delta, ctx);
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
  BallInPlungeLane,
  BallNotInPlungeLane,
}
