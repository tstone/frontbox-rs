pub use crate::prelude::*;

/// Simple system to automatically fire the plunger coil on `FirePlunger` command
pub struct AutoPlunger {
  lane_switch: &'static str,
  coil: &'static str,
  do_autoplunge: bool,
}

impl AutoPlunger {
  pub fn new(lane_switch: &'static str, coil: &'static str) -> Self {
    Self {
      lane_switch,
      coil,
      do_autoplunge: false,
    }
  }

  /// Fire the autoplunger immediately
  fn activate(&mut self, ctx: &Context, systems: &Systems) {
    systems
      .expect::<Machine>()
      .activate_driver(self.coil, ActivationMode::Tap, ctx);
  }

  /// Fire the autoplunger once the ball is resting in the lane
  pub fn fire(&mut self, ctx: &Context, systems: &Systems) {
    // Check the lane switch first to make sure the ball is ready
    if ctx.switches.is_closed(self.lane_switch) == Some(true) {
      self.activate(ctx, systems);
    } else {
      self.do_autoplunge = true;
    }
  }
}

impl System for AutoPlunger {
  fn on_startup(&mut self, ctx: &Context, systems: &Systems) {
    // Configure a meaty debounce to make sure the ball is fully resting on the forks
    let inverted = ctx.switches.config(self.lane_switch);

    let machine = systems.expect::<Machine>();

    machine.configure_switch(
      self.lane_switch,
      inverted.map(|c| c.inverted).unwrap_or(false),
      Some(Duration::from_millis(250)),
      None, // use default
      ctx,
    );

    // configure plunger driver
    let machine = systems.expect::<Machine>();
    machine.configure_driver(
      self.coil,
      PulseKickMode {
        initial_pwm_length: Duration::from_millis(50),
        initial_pwm_power: Power::percent(75),
        kick_length: Duration::from_millis(300),
        ..Default::default()
      },
      ctx,
    );
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context, systems: &Systems) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      if e.switch.name == self.lane_switch && self.do_autoplunge {
        self.activate(ctx, systems);
        self.do_autoplunge = false;
      }
    }
  }
}
