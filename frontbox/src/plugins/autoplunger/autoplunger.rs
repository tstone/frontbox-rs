pub use crate::prelude::*;

/// Simple system to automatically fire the plunger coil on `FirePlunger` command
pub struct AutoPlunger {
  lane_switch_query: HardwareQuery,
  coil_query: HardwareQuery,
  lane_switch_name: &'static str,
  coil_name: &'static str,
  do_autoplunge: bool,
}

impl AutoPlunger {
  pub fn new(lane_switch_query: HardwareQuery, coil_query: HardwareQuery) -> Self {
    Self {
      lane_switch_query,
      coil_query,
      do_autoplunge: false,
      lane_switch_name: "",
      coil_name: "",
    }
  }

  pub fn is_ball_in_trough(&self, ctx: &Context) -> bool {
    if ctx.switches.is_closed(self.lane_switch_name) == Some(true) {
      return true;
    }
    false
  }

  /// Fire the autoplunger immediately
  fn activate(&mut self, ctx: &Context) {
    ctx.activate_driver(self.coil_name, ActivationMode::Tap);
  }

  /// Fire the autoplunger once the ball is resting in the lane
  pub fn fire(&mut self, ctx: &Context) {
    // Check the lane switch first to make sure the ball is ready
    if ctx.switches.is_closed(self.lane_switch_name) == Some(true) {
      self.activate(ctx);
    } else {
      self.do_autoplunge = true;
    }
  }
}

impl System for AutoPlunger {
  fn on_spawn(&mut self, ctx: &Context) {
    self.lane_switch_name = self.lane_switch_query.get_switch_names(ctx)[0];
    self.coil_name = self.coil_query.get_driver_names(ctx)[0];

    // Configure a meaty debounce to make sure the ball is fully resting on the forks
    let inverted = ctx.switches.config(self.lane_switch_name);
    ctx.configure_switch(
      self.lane_switch_name,
      inverted.map(|c| c.inverted).unwrap_or(false),
      Some(Duration::from_millis(250)),
      None, // use default
    );

    // configure plunger driver
    ctx.configure_driver(
      self.coil_name,
      PulseKickMode {
        initial_pwm_length: Duration::from_millis(50),
        initial_pwm_power: Power::percent(75),
        kick_length: Duration::from_millis(300),
        ..Default::default()
      },
    );
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      if self.lane_switch_query.matches_switch(&e.switch) && self.do_autoplunge {
        self.activate(ctx);
        self.do_autoplunge = false;
      }
    }
  }
}
