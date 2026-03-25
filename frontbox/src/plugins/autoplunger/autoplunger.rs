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

  fn activate_plunger(&mut self, ctx: &mut Context) {
    ctx.command(ActivateDriver::new(self.coil, ActivationMode::Tap));
  }

  fn on_plunge(&mut self, ctx: &mut Context) {
    // Check the lane switch first to make sure the ball is ready
    let switch_lookup = ctx.expect::<SwitchLookup>();
    if switch_lookup.is_closed(self.lane_switch) == Some(true) {
      self.activate_plunger(ctx);
    } else {
      self.do_autoplunge = true;
    }
  }
}

impl System for AutoPlunger {
  fn on_startup(&mut self, ctx: &mut Context, _systems: &mut Systems) {
    ctx.register_command::<FirePlunger>();

    // Configure a meaty debounce to make sure the ball is fully resting on the forks
    let switch_lookup = ctx.expect::<SwitchLookup>();
    let inverted = switch_lookup.get_switch_config(self.lane_switch);
    ctx.command(ConfigureSwitch::new(
      self.lane_switch,
      inverted.map(|c| c.inverted).unwrap_or(false),
      Some(Duration::from_millis(250)),
      None, // use default
    ));

    // configure plunger driver
    ctx.command(ConfigureDriver::new(
      self.coil,
      PulseKickMode {
        initial_pwm_length: Duration::from_millis(50),
        initial_pwm_power: Power::percent(75),
        kick_length: Duration::from_millis(300),
        ..Default::default()
      },
    ));
  }

  fn on_command(&mut self, command: &dyn Signal, ctx: &mut Context) {
    if let Some(_fire) = command.downcast_ref::<FirePlunger>() {
      self.on_plunge(ctx);
    }
  }

  fn on_event(&mut self, event: &dyn Signal, ctx: &mut Context, _systems: &mut Systems) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      if e.switch.name == self.lane_switch && self.do_autoplunge {
        self.activate_plunger(ctx);
        self.do_autoplunge = false;
      }
    }
  }
}

// -- Commands --

/// This command can be queued up at any time, and will take effect once the ball is resting on the lane switch
pub struct FirePlunger;
