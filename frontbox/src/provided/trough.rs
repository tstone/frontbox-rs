use crate::prelude::*;

/// This system will monitor the specified switches to track the occupancy of the trough, and fire the eject coil when the trough is full and a new ball enters.
///
/// ## Events
/// - `BallEnteredTrough` - Emitted when a ball enters the trough
/// - `BallExitedTrough` - Emitted when a ball exits the trough
/// - `TroughFull` - Emitted when the trough reaches full occupancy
pub struct TroughSystem {
  handle: SystemHandle,
  switch_names: Vec<&'static str>,
  eject_coil_name: &'static str,
  /// The expected number of balls (might be decreased due to locked balls)
  expected_occupancy: usize,
  /// The last known amount of balls in the trough (used to compare if it has increased or decreased)
  last_recorded_occupancy: usize,
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
      last_recorded_occupancy: 0,
      handle: SystemHandle::default(),
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
          Power::HALF,
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
  }

  pub fn switch_definition(name: &'static str) -> SwitchDefinitionBuilder {
    SwitchDefinitionBuilder::new(name).debounce_close(Duration::from_millis(250))
  }

  fn on_trough_switch(&mut self, ctx: &SystemContext) {
    let occupancy = self.current_occupancy(ctx.into());
    let closed_count = occupancy.iter().filter(|b| **b).count();

    log::debug!(
      target: "frontbox::trough",
      "Re-evaluating occupancy: {} vs {} {:?}",
      closed_count,
      self.last_recorded_occupancy,
      occupancy
    );

    if closed_count > self.last_recorded_occupancy {
      log::debug!(target: "frontbox::trough", "Ball entered trough, occupancy: {:?}", occupancy);

      let is_full = occupancy.iter().all(|&o| o);
      ctx.emit(BallEnteredTrough::new(occupancy));

      if is_full {
        ctx.emit(TroughFull);
      }
      self.last_recorded_occupancy = closed_count;
    } else if closed_count < self.last_recorded_occupancy {
      log::debug!(target: "frontbox::trough", "Ball exited trough, occupancy: {:?}", occupancy);
      ctx.emit(BallExitedTrough::new(occupancy));
      self.last_recorded_occupancy = closed_count;
    }
  }

  fn current_occupancy_iter<'a>(&'a self, ctx: &'a ServiceContext) -> Box<dyn Iterator<Item = bool> + 'a> {
    Box::new(self
      .switch_names
      .iter()
      .take(self.expected_occupancy)
      .map(|name| ctx.for_system(self.handle).switches.is_closed(*name).unwrap() ))
  }

  pub fn current_occupancy(&self, ctx: &ServiceContext) -> Vec<bool> {
    self.current_occupancy_iter(ctx).collect()
  }

  pub fn current_occupancy_count(&self, ctx: &ServiceContext) -> usize {
    self.current_occupancy_iter(ctx).filter(|b| *b).count()
  }

  pub fn eject(&self, ctx: &ServiceContext) {
    let sctx = ctx.for_system(self.handle);
    sctx.activate_driver(self.eject_coil_name, ActivationMode::Tap);
    sctx.emit(BallExitedTrough::new(self.current_occupancy(ctx)));
  }

  /// Called when a ball has been removed from play (e.g. locked or stuck)
  pub fn ball_removed_from_play(&mut self) {
    if self.expected_occupancy > 0 {
      self.expected_occupancy -= 1;
    }
  }

  /// Called when a ball was previously removed from play, but has been re-added (e.g. locked ball released)
  pub fn ball_readded_to_play(&mut self) {
    let max_occupancy = self.switch_names.len();
    self.expected_occupancy = (self.expected_occupancy + 1).min(max_occupancy);
  }

  /// When called, the trough will examine current contents and use this to establish the expected amount of balls 
  /// present, which is the requirement for the `TroughFull` event. The intention is that this would be called at the
  /// beginning of a game, which would account for balls which maybe got stuck during play.
  pub fn establish_ball_occupancy(&mut self, ctx: &ServiceContext) {
    self.expected_occupancy = self.current_occupancy_count(ctx);
    self.last_recorded_occupancy = self.expected_occupancy;
  }
}

impl System for TroughSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = *ctx.current_handle();
    self.establish_ball_occupancy(ctx.into());
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      if self.switch_names.contains(&e.switch.name) {
        self.on_trough_switch(ctx);
      }
    } else if let Some(e) = event.downcast_ref::<SwitchOpened>() {
      if self.switch_names.contains(&e.switch.name) {
        self.on_trough_switch(ctx);
      }
    }
  }
}

// -- Events --

#[derive(serde::Serialize, Event)]
pub struct TroughFull;

#[derive(serde::Serialize, Event)]
pub struct BallEnteredTrough {
  pub occupancy: Vec<bool>,
}

impl BallEnteredTrough {
  pub fn new(occupancy: Vec<bool>) -> BallEnteredTrough {
    Self { occupancy }
  }
}

#[derive(serde::Serialize, Event)]
pub struct BallExitedTrough {
  pub occupancy: Vec<bool>,
}

impl BallExitedTrough {
  pub fn new(occupancy: Vec<bool>) -> BallExitedTrough {
    Self { occupancy }
  }
}

#[cfg(test)]
mod tests {
  use fast_protocol::SwitchState;

use super::*;

  #[test]
  fn current_occupancy() {
    let system = TroughSystem::new("eject", vec!["a", "b", "c"]);
    let mut context = TestContext::default();
    context.insert_switch(Switch { name: "a", id: 0, ..Default::default() });
    context.insert_switch(Switch { name: "b", id: 1, ..Default::default() });
    context.insert_switch(Switch { name: "c", id: 2, ..Default::default() });
    
    context.base.switches.update_switch_states(vec![SwitchState::Closed, SwitchState::Closed, SwitchState::Open]);
    let occupancy = system.current_occupancy(&context.svc_ctx());
    assert_eq!(occupancy, vec![true, true, false]);
  }

  #[test]
  fn current_occupancy_count() {
    let system = TroughSystem::new("eject", vec!["a", "b", "c"]);
    let mut context = TestContext::default();
    context.insert_switch(Switch { name: "a", id: 0, ..Default::default() });
    context.insert_switch(Switch { name: "b", id: 1, ..Default::default() });
    context.insert_switch(Switch { name: "c", id: 2, ..Default::default() });
    
    context.base.switches.update_switch_states(vec![SwitchState::Closed, SwitchState::Closed, SwitchState::Closed]);
    let count = system.current_occupancy_count(&context.svc_ctx());
    assert_eq!(count, 3);

    context.base.switches.update_switch_states(vec![SwitchState::Closed, SwitchState::Closed, SwitchState::Open]);
    let count = system.current_occupancy_count(&context.svc_ctx());
    assert_eq!(count, 2);
  }

  #[test]
  fn event_trough_switch_opened_closed() {
    let mut system = TroughSystem::new("eject", vec!["a", "b", "c"]);
    let mut context = TestContext::default();
    let target_switch = Switch { name: "c", id: 2, ..Default::default() };
    context.insert_switch(Switch { name: "a", id: 0, ..Default::default() });
    context.insert_switch(Switch { name: "b", id: 1, ..Default::default() });
    context.insert_switch(target_switch.clone());
    context.base.switches.update_switch_states(vec![SwitchState::Closed, SwitchState::Closed, SwitchState::Closed]);
    system.on_spawn(&context.sys_ctx());

    // opened (ball left)
    context.base.switches.update_switch_states(vec![SwitchState::Closed, SwitchState::Closed, SwitchState::Open]);
    system.on_event(&SwitchOpened::new(target_switch.clone()), &context.sys_ctx());

    let events = context.events_emitted();
    assert_eq!(events[0].short_name(), "BallExitedTrough");
    let count = system.current_occupancy_count(&context.svc_ctx());
    assert_eq!(count, 2);

    // closed (ball re-entered)
    context.base.switches.update_switch_states(vec![SwitchState::Closed, SwitchState::Closed, SwitchState::Closed]);
    system.on_event(&SwitchClosed::new(target_switch), &context.sys_ctx());

    let events = context.events_emitted();
    assert_eq!(events[0].short_name(), "BallEnteredTrough");
    assert_eq!(events[1].short_name(), "TroughFull");

    let count = system.current_occupancy_count(&context.svc_ctx());
    assert_eq!(count, 3);
  }

  #[test]
  fn establish_ball_occupancy() {
    let mut system = TroughSystem::new("eject", vec!["a", "b", "c"]);
    let mut context = TestContext::default();
    let switch2 = Switch { name: "b", id: 1, ..Default::default() };
    let switch3 = Switch { name: "c", id: 2, ..Default::default() };
    context.insert_switch(Switch { name: "a", id: 0, ..Default::default() });
    context.insert_switch(switch2.clone());
    context.insert_switch(switch3.clone());
    context.base.switches.update_switch_states(vec![SwitchState::Closed, SwitchState::Closed, SwitchState::Closed]);
    system.on_spawn(&context.sys_ctx());

    // ball #1 leaves
    context.base.switches.update_switch_states(vec![SwitchState::Closed, SwitchState::Closed, SwitchState::Open]);
    system.on_event(&SwitchOpened::new(switch3.clone()), &context.sys_ctx());
    system.establish_ball_occupancy(&context.svc_ctx());

    // ball #2 leaves
    context.base.switches.update_switch_states(vec![SwitchState::Closed, SwitchState::Open, SwitchState::Open]);
    system.on_event(&SwitchOpened::new(switch2.clone()), &context.sys_ctx());

    let events = context.events_emitted();
    assert_eq!(events[0].short_name(), "BallExitedTrough");
    let count = system.current_occupancy_count(&context.svc_ctx());
    assert_eq!(count, 1);

    // ball #2 re-enters, now full because occupancy established at 2
    context.base.switches.update_switch_states(vec![SwitchState::Closed, SwitchState::Closed, SwitchState::Closed]);
    system.on_event(&SwitchClosed::new(switch2), &context.sys_ctx());

    let events = context.events_emitted();
    assert_eq!(events[0].short_name(), "BallEnteredTrough");
    assert_eq!(events[1].short_name(), "TroughFull");

    let count = system.current_occupancy_count(&context.svc_ctx());
    assert_eq!(count, 2);
  }

  #[test]
  fn ball_added_removed_from_play() {
    let mut system = TroughSystem::new("eject", vec!["a", "b", "c"]);
    let mut context = TestContext::default();
    let switch2 = Switch { name: "b", id: 1, ..Default::default() };
    let switch3 = Switch { name: "c", id: 2, ..Default::default() };
    context.insert_switch(Switch { name: "a", id: 0, ..Default::default() });
    context.insert_switch(switch2.clone());
    context.insert_switch(switch3.clone());
    context.base.switches.update_switch_states(vec![SwitchState::Closed, SwitchState::Closed, SwitchState::Closed]);
    system.on_spawn(&context.sys_ctx());

    system.ball_removed_from_play(); // now expecting 2 balls instead of 3

    // ball #1 leaves
    context.base.switches.update_switch_states(vec![SwitchState::Closed, SwitchState::Closed, SwitchState::Open]);
    system.on_event(&SwitchOpened::new(switch3.clone()), &context.sys_ctx());

    let events = context.events_emitted();
    assert_eq!(events[0].short_name(), "BallExitedTrough");
    let count = system.current_occupancy_count(&context.svc_ctx());
    assert_eq!(count, 2);

    // ball #2 leaves
    context.base.switches.update_switch_states(vec![SwitchState::Closed, SwitchState::Open, SwitchState::Open]);
    system.on_event(&SwitchOpened::new(switch2.clone()), &context.sys_ctx());

    let events = context.events_emitted();
    assert_eq!(events[0].short_name(), "BallExitedTrough");
    let count = system.current_occupancy_count(&context.svc_ctx());
    assert_eq!(count, 1);

    // ball #2 re-enters, now full because of ball removed from play
    context.base.switches.update_switch_states(vec![SwitchState::Closed, SwitchState::Closed, SwitchState::Closed]);
    system.on_event(&SwitchClosed::new(switch2), &context.sys_ctx());

    let events = context.events_emitted();
    assert_eq!(events[0].short_name(), "BallEnteredTrough");
    assert_eq!(events[1].short_name(), "TroughFull");

    let count = system.current_occupancy_count(&context.svc_ctx());
    assert_eq!(count, 2);
  }
}
