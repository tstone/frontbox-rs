use crate::plugins::TroughPlugin;
pub use crate::prelude::*;

pub struct Trough {
  pub switches: Vec<&'static str>,
  pub eject_coil: &'static str,
  pub expected_occupancy: usize,
}

impl Trough {
  pub fn new(switches: Vec<&'static str>, eject_coil: &'static str) -> Self {
    Self {
      expected_occupancy: switches.len(),
      switches,
      eject_coil,
    }
  }

  fn on_trough_switch_closed(&mut self, switch_name: &str, ctx: &Context) {
    if self
      .switches
      // only look at the last switch (nearest the exit) for occupancy changes
      .get(self.expected_occupancy - 1)
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

  fn on_trough_switch_opened(&mut self, switch_name: &str, ctx: &Context) {
    if self
      .switches
      // only look at the last switch (nearest the exit) for occupancy changes
      .get(self.expected_occupancy - 1)
      .map(|s| *s == switch_name)
      .unwrap_or(false)
    {
      let occupancy = self.get_occupancy(ctx);
      log::debug!("Ball exited trough, occupancy: {:?}", occupancy);
      ctx.emit(BallExitedTrough::new(occupancy));
    }
  }

  fn get_occupancy(&self, ctx: &Context) -> Vec<bool> {
    let mut occupancy = Vec::new();
    for (_, switch) in self
      .switches
      .iter()
      .enumerate()
      .take(self.expected_occupancy)
    {
      occupancy.push(ctx.switches.is_closed(switch).unwrap());
    }

    occupancy
  }

  pub fn eject(&self, ctx: &Context, systems: &Systems) {
    systems
      .expect::<Machine>()
      .activate_driver(self.eject_coil, ActivationMode::Tap, ctx);
  }

  pub fn ball_added_to_play(&mut self) {
    let max_occupancy = self.switches.len();
    if self.expected_occupancy < max_occupancy {
      self.expected_occupancy += 1;
    }
  }

  pub fn ball_removed_from_play(&mut self) {
    if self.expected_occupancy > 0 {
      self.expected_occupancy -= 1;
    }
  }
}

impl System for Trough {
  fn on_startup(&mut self, ctx: &Context, systems: &Systems) {
    // configure switch debounce to be long to avoid triggering events as the ball rolls down the trough and hits multiple switches in quick succession.
    let machine = systems.expect::<Machine>();

    for switch in &self.switches {
      // preserve configured inverted settings (if present)
      let inverted = ctx
        .switches
        .config(switch)
        .map(|c| c.inverted)
        .unwrap_or(false);

      machine.configure_switch(
        switch,
        inverted,
        Some(Duration::from_millis(250)),
        None,
        ctx,
      );
    }

    // configure eject driver
    let operator_config = systems.expect::<OperatorConfig>();
    let trough_kick_len = operator_config
      .get_integer(TroughPlugin::config().trough_kick)
      .unwrap_or(100);
    let trough_init_power = operator_config
      .get_integer(TroughPlugin::config().trough_power)
      .unwrap_or(70);

    machine.configure_driver(
      self.eject_coil,
      PulseKickMode {
        initial_pwm_length: Duration::from_millis(50),
        initial_pwm_power: Power::percent(trough_init_power as u8),
        kick_length: Duration::from_millis(trough_kick_len as u64),
        ..Default::default()
      },
      ctx,
    );
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context, _systems: &Systems) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      self.on_trough_switch_closed(&e.switch.name, ctx);
    } else if let Some(e) = event.downcast_ref::<SwitchOpened>() {
      self.on_trough_switch_opened(&e.switch.name, ctx);
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
