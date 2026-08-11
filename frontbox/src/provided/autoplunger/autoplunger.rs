use crate::operator_config::*;
use crate::prelude::*;
use crate::provided::BallEnteredPlungeLane;
use crate::provided::PlungeLaneState;
use crate::provided::PlungeLaneSystem;

/// Simple system to manage firing the plunger eject coil
pub struct AutoPlungerSystem {
  coil_name: &'static str,
  do_autoplunge: bool,
}

impl AutoPlungerSystem {
  pub fn new(coil_name: &'static str) -> Self {
    Self {
      do_autoplunge: false,
      coil_name,
    }
  }

  pub fn coil_definition(name: &'static str) -> DriverDefinitionBuilder {
    DriverDefinitionBuilder::new(name).mode(PulseKickMode {
      initial_pwm_length: HardwareValue::config(
        "Autoplunger Touch Time",
        "Duration by which the forks are brought into contact with the ball, before full launch",
        Duration::from_millis(7),
        Ranges::duration(0, 100),
      ),
      initial_pwm_power: HardwareValue::fixed(Power::HALF),
      secondary_pwm_power: HardwareValue::Fixed(Power::ZERO),
      secondary_pwm_length: HardwareValue::Fixed(Duration::ZERO),
      kick_length: HardwareValue::config(
        "Autoplunger Coil Launch Time",
        "Duration that the forks exert full power onto the ball (kick)",
        Duration::from_millis(24),
        Ranges::duration(5, 75),
      ),
      ..Default::default()
    })
  }

  /// Fire the autoplunger immediately
  fn activate_coil(&self, ctx: &SystemContext) {
    ctx.activate_driver(self.coil_name, ActivationMode::Tap);
  }

  /// Fire the autoplunger once the ball is resting in the lane
  pub fn fire(&mut self, ctx: &SystemContext) {
    // Check that the ball is present
    let plunge_lane = ctx.expect::<PlungeLaneSystem>();
    if plunge_lane.is_ball_present() {
      self.activate_coil(ctx);
    } else {
      // queue it up for when the ball is present
      self.do_autoplunge = true;
    }
  }
}

impl System for AutoPlungerSystem {
  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(event) = event.downcast_ref::<BallEnteredPlungeLane>() {
      if self.do_autoplunge {
        self.activate_coil(ctx);
        self.do_autoplunge = false;
      } else if event.state == PlungeLaneState::UnexpectedBallPresent {
        log::debug!("Firing auto-plunger due to unexpected ball in plunge lane");
        // Automatically launch if it wasn't an expected ball
        self.fire(ctx);
      }
    }
  }
}
