use uuid::Uuid;

use crate::prelude::*;

/// This system will monitor the specified switches to track the occupancy of the trough, and fire the eject coil when the trough is full and a new ball enters.
///
/// ## Events
/// - `BallEnteredTrough` - Emitted when a ball enters the trough
/// - `BallExitedTrough` - Emitted when a ball exits the trough
/// - `TroughFull` - Emitted when the trough reaches full occupancy
pub struct Trough {
  switch_names: Vec<&'static str>,
  eject_coil_name: &'static str,
  expected_occupancy: usize,
}

impl Trough {
  pub fn new(switch_count: usize) -> Self {
    Self {
      switch_names: (0..switch_count)
        .map(|i| -> &'static str {
          Box::leak(format!("trough_switchswitch_{i}_{}", Uuid::new_v4()).into_boxed_str())
        })
        .collect(),
      eject_coil_name: Box::leak(format!("trough_eject_coil_{}", Uuid::new_v4()).into_boxed_str()),
      expected_occupancy: 0,
    }
  }

  pub fn eject_coil_definition(&self) -> DriverDefinitionBuilder {
    DriverDefinitionBuilder::new(self.eject_coil_name)
      .mode(PulseKickMode {
        initial_pwm_length: HardwareValue::config(
          "Trough Plunger Touch Time",
          "Duration by which the eject plunger is brought into contact with the ball, before full eject",
          Duration::from_millis(7),
          Ranges::duration(0, 100),
        ),
        initial_pwm_power: HardwareValue::fixed(
          Power::percent(75),
        ),
        secondary_pwm_power: HardwareValue::Fixed(Power::ZERO),
        secondary_pwm_length: HardwareValue::Fixed(Duration::ZERO),
        kick_length: HardwareValue::config(
          "Trough Eject Time",
          "Duration that the plunger exert full power onto the ball (kick)",
          Duration::from_millis(75),
          Ranges::duration(10, 300),
        ),
        ..Default::default()
      })
      .tag(tags::Trough)
  }

  pub fn switch_definition(&self, index: usize) -> SwitchDefinitionBuilder {
    SwitchDefinitionBuilder::new(self.switch_names[index])
      .debounce_open(Duration::from_millis(250))
      .tag(tags::Trough)
  }

  fn on_trough_switch_closed(&mut self, switch_name: &str, ctx: &Context) {
    if self
      .switch_names
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
      .switch_names
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

impl System for Trough {
  fn on_spawn(&mut self, _ctx: &Context) {
    self.expected_occupancy = self.switch_names.len();
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
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
