pub use crate::prelude::*;

pub struct TroughSystem {
  pub switches: Vec<&'static str>,
  pub eject_coil: &'static str,
  pub expected_occupancy: usize,
}

impl TroughSystem {
  /// # Arguments
  /// * `switches` - List of trough switches, in order. Index 0 is the switch nearest the exit.
  pub fn new(switches: Vec<&'static str>, eject_coil: &'static str) -> Box<Self> {
    Box::new(Self {
      expected_occupancy: switches.len(),
      switches,
      eject_coil,
    })
  }

  fn on_trough_switch_closed(&mut self, switch_name: &str, ctx: &mut Context) {
    if self
      .switches
      // only look at the last switch (nearest the exit) for occupancy changes
      .get(self.expected_occupancy)
      .map(|s| *s == switch_name)
      .unwrap_or(false)
    {
      let occupancy = self.get_occupancy(ctx);
      log::debug!("Ball entered trough, occupancy: {:?}", occupancy);
      let is_full = occupancy.iter().all(|&o| o);
      ctx.emit(BallEnteredTrough::new(occupancy));
      if is_full {
        ctx.emit(TroughFull);
      }
    }
  }

  fn on_trough_switch_opened(&mut self, switch_name: &str, ctx: &mut Context) {
    if self
      .switches
      // only look at the last switch (nearest the exit) for occupancy changes
      .get(self.expected_occupancy)
      .map(|s| *s == switch_name)
      .unwrap_or(false)
    {
      let occupancy = self.get_occupancy(ctx);
      log::debug!("Ball exited trough, occupancy: {:?}", occupancy);
      ctx.emit(BallExitedTrough::new(occupancy));
    }
  }

  fn get_occupancy(&self, ctx: &mut Context) -> Vec<bool> {
    let switch_lookup = ctx.expect::<SwitchLookup>();
    let mut occupancy = Vec::new();
    for (_, switch) in self
      .switches
      .iter()
      .enumerate()
      .take(self.expected_occupancy)
    {
      occupancy.push(switch_lookup.is_closed(switch).unwrap());
    }

    occupancy
  }

  fn eject(&self, ctx: &mut Context) {
    ctx.command(ActivateDriver::new(self.eject_coil, ActivationMode::Tap));
  }

  fn ball_added_to_play(&mut self) {
    let max_occupancy = self.switches.len();
    if self.expected_occupancy < max_occupancy {
      self.expected_occupancy += 1;
    }
  }

  fn ball_removed_from_play(&mut self) {
    if self.expected_occupancy > 0 {
      self.expected_occupancy -= 1;
    }
  }
}

impl System for TroughSystem {
  fn on_startup(&mut self, ctx: &mut Context) {
    ctx.register_command::<TroughEject>();
    ctx.register_command::<BallAddedToPlay>();
    ctx.register_command::<BallRemovedFromPlay>();

    // configure switch debounce to be long to avoid triggering events as the ball rolls down the trough and hits multiple switches in quick succession.
    let switch_lookup = ctx.expect::<SwitchLookup>();
    let mut switch_cmds = Vec::new();
    for switch in &self.switches {
      // preserve configured inverted settings (if present)
      let inverted = switch_lookup
        .get_switch_config(switch)
        .map(|c| c.inverted)
        .unwrap_or(false);
      switch_cmds.push(ConfigureSwitch::new(
        switch,
        inverted,
        Some(Duration::from_millis(500)),
        None, // use default
      ));
    }
    for cmd in switch_cmds {
      ctx.command(cmd);
    }

    // configure eject driver
    // TODO: confirm these values
    ctx.command(ConfigureDriver::new(
      self.eject_coil,
      PulseKickMode {
        initial_pwm_length: Duration::from_millis(50),
        initial_pwm_power: Power::percent(75),
        kick_length: Duration::from_millis(100),
        ..Default::default()
      },
    ));
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      self.on_trough_switch_closed(&e.switch.name, ctx);
    } else if let Some(e) = event.downcast_ref::<SwitchOpened>() {
      self.on_trough_switch_opened(&e.switch.name, ctx);
    }
  }

  fn on_command(&mut self, command: &dyn Command, ctx: &mut Context) {
    if let Some(_) = command.downcast_ref::<TroughEject>() {
      self.eject(ctx);
    } else if let Some(_) = command.downcast_ref::<BallAddedToPlay>() {
      self.ball_added_to_play();
    } else if let Some(_) = command.downcast_ref::<BallRemovedFromPlay>() {
      self.ball_removed_from_play();
    }
  }
}

// -- Commands --

pub struct TroughEject;

/// This command causes the trough to expect one less ball in it's occupancy calculations. This is typically called in situations where something like a physical ball lock is holding onto a ball that should no longer be expected in the trough.
pub struct BallRemovedFromPlay;
/// This command causes the trough to expect one more ball in it's occupancy calculations. This is typically called in situations where a ball is added back into play, such as when a ball is released from a physical lock.
pub struct BallAddedToPlay;

// -- Events --

pub struct TroughFull;

#[derive(Debug)]
#[allow(unused)]
pub struct BallEnteredTrough {
  pub occupancy: Vec<bool>,
}

impl BallEnteredTrough {
  pub fn new(occupancy: Vec<bool>) -> Box<BallEnteredTrough> {
    Box::new(Self { occupancy })
  }
}

#[derive(Debug)]
#[allow(unused)]
pub struct BallExitedTrough {
  pub occupancy: Vec<bool>,
}

impl BallExitedTrough {
  pub fn new(occupancy: Vec<bool>) -> Box<BallExitedTrough> {
    Box::new(Self { occupancy })
  }
}
