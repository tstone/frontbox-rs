use crate::operator_config::*;
use crate::prelude::*;
use uuid::Uuid;

/// Simple system to manage firing the plunger eject coil
pub struct AutoPlunger {
  lane_switch_name: &'static str,
  coil_name: &'static str,
  do_autoplunge: bool,
}

impl AutoPlunger {
  pub fn new() -> Self {
    Self {
      do_autoplunge: false,
      lane_switch_name: Box::leak(
        format!("autoplunger_lane_switch_{}", Uuid::new_v4()).into_boxed_str(),
      ),
      coil_name: Box::leak(format!("autoplunger_coil_{}", Uuid::new_v4()).into_boxed_str()),
    }
  }

  pub fn switch_definition(&self) -> SwitchDefinitionBuilder {
    // Configure a meaty debounce to make sure the ball is fully resting on the forks
    SwitchDefinitionBuilder::new(self.lane_switch_name).debounce_open(Duration::from_millis(250))
  }

  pub fn coil_definition(&self) -> DriverDefinitionBuilder {
    DriverDefinitionBuilder::new(self.coil_name).mode(PulseKickMode {
      initial_pwm_length: HardwareValue::config(
        "Autoplunger Touch Time",
        "Duration by which the forks are brought into contact with the ball, before full launch",
        Duration::from_millis(7),
        Ranges::duration(0, 100),
      ),
      initial_pwm_power: HardwareValue::fixed(Power::percent(75)),
      secondary_pwm_power: HardwareValue::Fixed(Power::ZERO),
      secondary_pwm_length: HardwareValue::Fixed(Duration::ZERO),
      kick_length: HardwareValue::config(
        "Autoplunger Coil Launch Time",
        "Duration that the forks exert full power onto the ball (kick)",
        Duration::from_millis(100),
        Ranges::duration(10, 300),
      ),
      ..Default::default()
    })
  }

  pub fn is_ball_in_trough(&self, ctx: &Context) -> bool {
    if ctx.switches.is_closed(&self.lane_switch_name) == Some(true) {
      return true;
    }
    false
  }

  /// Fire the autoplunger immediately
  fn activate(&self, ctx: &Context) {
    ctx.activate_driver(self.coil_name, ActivationMode::Tap);
  }

  /// Fire the autoplunger once the ball is resting in the lane
  pub fn fire(&mut self, ctx: &Context) {
    // Check the lane switch first to make sure the ball is ready
    if ctx.switches.is_closed(&self.lane_switch_name) == Some(true) {
      self.activate(ctx);
    } else {
      self.do_autoplunge = true;
    }
  }
}

impl System for AutoPlunger {
  fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      if self.lane_switch_name.eq(e.switch.name) && self.do_autoplunge {
        self.activate(ctx);
        self.do_autoplunge = false;
      }
    }
  }
}
