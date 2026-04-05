pub use crate::prelude::*;

/// Simple system to automatically fire the plunger coil on `FirePlunger` command
pub struct AutoPlunger {
  lane_switch: HardwareSelection,
  coil: HardwareSelection,
  do_autoplunge: bool,
}

impl AutoPlunger {
  pub fn new(lane_switch: HardwareSelection, coil: HardwareSelection) -> Self {
    Self {
      lane_switch,
      coil,
      do_autoplunge: false,
    }
  }

  /// Fire the autoplunger immediately
  fn activate(&mut self, ctx: &Context, systems: &Systems) {
    for driver in self.coil.get_drivers(ctx) {
      systems
        .expect::<Machine>()
        .activate_driver(driver.name, ActivationMode::Tap, ctx);
    }
  }

  /// Fire the autoplunger once the ball is resting in the lane
  pub fn fire(&mut self, ctx: &Context, systems: &Systems) {
    // Check the lane switch first to make sure the ball is ready
    let is_closed = self
      .lane_switch
      .get_switches(ctx)
      .iter()
      .all(|s| ctx.switches.is_closed(*&s.name) == Some(true));

    if is_closed {
      self.activate(ctx, systems);
    } else {
      self.do_autoplunge = true;
    }
  }
}

impl System for AutoPlunger {
  fn on_startup(&mut self, ctx: &Context, systems: &Systems) {
    let machine = systems.expect::<Machine>();

    // Configure a meaty debounce to make sure the ball is fully resting on the forks
    for switch in self.lane_switch.get_switches(ctx) {
      let inverted = ctx.switches.config(switch.name);
      machine.configure_switch(
        switch.name,
        inverted.map(|c| c.inverted).unwrap_or(false),
        Some(Duration::from_millis(250)),
        None, // use default
        ctx,
      );
    }

    // configure plunger driver
    let machine = systems.expect::<Machine>();
    for driver in self.coil.get_drivers(ctx) {
      machine.configure_driver(
        driver.name,
        PulseKickMode {
          initial_pwm_length: Duration::from_millis(50),
          initial_pwm_power: Power::percent(75),
          kick_length: Duration::from_millis(300),
          ..Default::default()
        },
        ctx,
      );
    }
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context, systems: &Systems) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      if self.lane_switch.matches_switch(&e.switch) && self.do_autoplunge {
        self.activate(ctx, systems);
        self.do_autoplunge = false;
      }
    }
  }
}
