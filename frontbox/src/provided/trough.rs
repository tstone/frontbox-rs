use crate::prelude::*;

/// This system will monitor the specified switches to track the occupancy of the trough, and fire the eject coil when the trough is full and a new ball enters.
///
/// ## Events
/// - `BallEnteredTrough` - Emitted when a ball enters the trough
/// - `BallExitedTrough` - Emitted when a ball exits the trough
/// - `TroughFull` - Emitted when the trough reaches full occupancy
pub struct TroughSystem {
  switch_names: Vec<&'static str>,
  eject_coil_name: &'static str,
  expected_occupancy: usize,
  last_occupancy: usize,
}

impl TroughSystem {
  /// * `eject_coil_name` - Name of trough eject coil (driver)
  /// * `switch_names` - Names of trough switches, in order. First switch is the switch nearest the exit.
  pub fn new(eject_coil_name: &'static str, switch_names: Vec<&'static str>) -> Self {
    Self {
      expected_occupancy: switch_names.len(),
      switch_names,
      eject_coil_name,
      // set on launch
      last_occupancy: 0,
    }
  }

  pub fn eject_coil_definition(name: &'static str) -> DriverDefinitionBuilder {
    DriverDefinitionBuilder::new(name)
      .mode(PulseKickMode {
        initial_pwm_length: HardwareValue::config(
          "Plunger Touch Time",
          "Duration by which the eject plunger is brought into contact with the ball, before full eject",
          Duration::from_millis(5),
          Ranges::duration(0, 50),
        ),
        initial_pwm_power: HardwareValue::fixed(
          Power::percent(40),
        ),
        secondary_pwm_power: HardwareValue::Fixed(Power::ZERO),
        secondary_pwm_length: HardwareValue::Fixed(Duration::ZERO),
        kick_length: HardwareValue::config(
          "Eject Time",
          "Duration that the plunger exert full power onto the ball (kick)",
          Duration::from_millis(12),
          Ranges::duration(5, 75),
        ),
        ..Default::default()
      })
      .tag(tags::Trough)
  }

  pub fn switch_definition(name: &'static str) -> SwitchDefinitionBuilder {
    SwitchDefinitionBuilder::new(name)
      .debounce_close(Duration::from_millis(250))
      .tag(tags::Trough)
  }

  fn on_switch(&mut self, switch_name: &str, ctx: &Context) {
    if self.switch_names.contains(&switch_name) {
      self.re_evaluate_occupancy(ctx);
    }
  }

  fn re_evaluate_occupancy(&mut self, ctx: &Context) {
    let occupancy = self.get_occupancy(ctx);
    let closed_count = occupancy.iter().filter(|b| **b).count();

    log::debug!(
      "Re-evaluating occupancy: {} vs {} {:?}",
      closed_count,
      self.last_occupancy,
      occupancy
    );

    if closed_count > self.last_occupancy {
      log::debug!("Ball entered trough, occupancy: {:?}", occupancy);

      let is_full = occupancy.iter().all(|&o| o);
      ctx.emit(BallEnteredTrough::new(occupancy));

      if is_full {
        ctx.emit(TroughFull);
      }
      self.last_occupancy = closed_count;
    } else if closed_count < self.last_occupancy {
      log::debug!("Ball exited trough, occupancy: {:?}", occupancy);
      ctx.emit(BallExitedTrough::new(occupancy));
      self.last_occupancy = closed_count;
    }
  }

  fn get_occupancy(&self, ctx: &Context) -> Vec<bool> {
    let mut occupancy = Vec::new();
    for (_, switch) in self
      .switch_names
      .iter()
      .enumerate()
      .take(self.expected_occupancy)
    {
      occupancy.push(ctx.switches.is_closed(switch).unwrap());
    }

    occupancy
  }

  pub fn eject(&self, ctx: &Context) {
    ctx.activate_driver(self.eject_coil_name, ActivationMode::Tap);
    ctx.emit(BallExitedTrough::new(self.get_occupancy(ctx)));
  }

  pub fn ball_added_to_play(&mut self) {
    let max_occupancy = self.switch_names.len();
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

impl System for TroughSystem {
  fn on_spawn(&mut self, ctx: &Context) {
    log::debug!("Trough spawn");
    self.last_occupancy = self.get_occupancy(ctx).iter().filter(|b| **b).count();
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      self.on_switch(&e.switch.name, ctx);
    } else if let Some(e) = event.downcast_ref::<SwitchOpened>() {
      self.on_switch(&e.switch.name, ctx);
    }
  }

  fn on_reactivate(&mut self, ctx: &Context) {
    // In case ball state changed while trough was deactivated (e.g. menu)
    // re-evaluate occupancy and emit events as necessary
    self.re_evaluate_occupancy(ctx);
  }
}

// -- Events --

pub struct TroughFull;

#[derive(Debug)]
#[allow(unused)]
pub struct BallEnteredTrough {
  pub occupancy: Vec<bool>,
}

impl BallEnteredTrough {
  pub fn new(occupancy: Vec<bool>) -> BallEnteredTrough {
    Self { occupancy }
  }
}

#[derive(Debug)]
#[allow(unused)]
pub struct BallExitedTrough {
  pub occupancy: Vec<bool>,
}

impl BallExitedTrough {
  pub fn new(occupancy: Vec<bool>) -> BallExitedTrough {
    Self { occupancy }
  }
}
