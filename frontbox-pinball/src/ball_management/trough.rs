use frontbox::prelude::*;

use crate::ball_management::ball_management_config::BallManagementConfig;
pub struct TroughSystem {
  config: BallManagementConfig,
  handle: SystemHandle,
  eject_state: EjectState,
  eject_queue: u8,
  eject_retries: u8,
  occupancy_state: OccupancyState,
  last_occupancy: u8,
  /// Ejects verified since occupancy was last reported, used to separate exits from entries
  exits_since_report: u8,
}

impl TroughSystem {
  pub fn new(config: BallManagementConfig) -> Self {
    Self {
      config,
      handle: SystemHandle::default(),
      eject_state: EjectState::Pending,
      eject_queue: 0,
      eject_retries: 0,
      occupancy_state: OccupancyState::Ready,
      last_occupancy: 0, // set on spawn
      exits_since_report: 0,
    }
  }

  pub fn eject(&mut self, ctx: &ServiceContext) {
    let ctx = &ctx.for_system(self.handle);

    // Wait for balls to stop moving, and for a ball to be present (a jammed ball can still be ejected)
    let settling = matches!(self.occupancy_state, OccupancyState::Settling { .. });
    let empty = self.last_occupancy == 0 && !self.is_jammed(ctx);
    if settling || empty || self.eject_state != EjectState::Ready {
      self.eject_queue += 1;
      return;
    }

    ctx.emit(TroughEjecting);
    self.eject_inner(ctx);
  }

  pub fn eject_many(&mut self, count: u8, ctx: &ServiceContext) {
    match count {
      0 => {}
      1 => self.eject(ctx),
      _ => {
        self.eject_queue += count - 1;
        self.eject(ctx);
      }
    }
  }

  /// True if the jam switch is configured and currently closed
  pub fn is_jammed(&self, ctx: &SystemContext) -> bool {
    self
      .config
      .jam_switch_name
      .map(|name| ctx.switches.is_closed(name).unwrap_or(false))
      .unwrap_or(false)
  }

  /// Number of balls in the trough as of the last time it settled
  pub fn occupancy(&self) -> u8 {
    self.last_occupancy
  }

  fn retry_eject(&mut self, ctx: &SystemContext) {
    if self.eject_retries >= self.config.eject_retries_max {
      log::warn!(target: "frontbox::trough", "Eject failed after {} retries", self.eject_retries);
      ctx.emit(TroughEjectFailed);
      self.eject_ready(ctx);
    } else {
      self.eject_retries += 1;
      log::info!(target: "frontbox::trough", "Eject not verified, retrying ({}/{})", self.eject_retries, self.config.eject_retries_max);

      if matches!(self.occupancy_state, OccupancyState::Settling { .. }) {
        // Balls are still moving; defer the retry until settled. Ready (without resetting retries) lets the queue run it.
        self.eject_state = EjectState::Ready;
        self.eject_queue += 1;
      } else {
        self.eject_inner(ctx);
      }
    }
  }

  fn eject_inner(&mut self, ctx: &SystemContext) {
    self.eject_state = EjectState::Verifying {
      target_occupancy: self.read_switch_occupancy(ctx.into()).saturating_sub(1),
      cue_id: ctx.cue(VerifyEject, self.config.eject_verification_time.once()),
    };
    ctx.activate_driver(self.config.eject_coil_name, ActivationMode::Tap);
  }

  fn read_switch_occupancy(&self, ctx: &SystemContext) -> u8 {
    self
      .config
      .trough_switch_names
      .iter()
      .filter(|name| ctx.switches.is_closed(**name).unwrap())
      .count() as u8
  }

  fn verify_eject_switch(&mut self, ctx: &SystemContext) {
    if let EjectState::Verifying { cue_id, .. } = self.eject_state {
      ctx.cancel_cue(cue_id);
      self.exits_since_report += 1;
      self.eject_ready(ctx);
    }
  }

  /// The preferred verification is the eject switch query, but if that times out
  /// this fallback looks at the trough occupancy (less good as it misses reentrant balls)
  fn verify_eject_fallback(&mut self, ctx: &SystemContext) {
    if let EjectState::Verifying {
      target_occupancy, ..
    } = self.eject_state
    {
      // A closed jam switch means the ball didn't leave, even if it no longer registers in occupancy
      if !self.is_jammed(ctx) && self.read_switch_occupancy(ctx.into()) <= target_occupancy {
        self.exits_since_report += 1;
        self.eject_ready(ctx);
      } else {
        self.retry_eject(ctx);
      }
    }
  }

  fn eject_ready(&mut self, ctx: &SystemContext) {
    self.eject_retries = 0;
    self.eject_state = EjectState::Ready;

    // A report held while this eject was verifying can go out now. If still settling, the settle will report instead.
    // This must happen before the next queued eject starts verifying and holds reporting again.
    if self.occupancy_state == OccupancyState::Ready {
      self.report_occupancy(ctx);
    }
    self.process_queued_ejects(ctx);
  }

  fn process_queued_ejects(&mut self, ctx: &SystemContext) {
    if self.eject_queue > 0 {
      self.eject_queue = self.eject_queue.saturating_sub(1);
      self.eject(ctx.into());
    }
  }

  fn queue_occupancy_settling(&mut self, ctx: &SystemContext) {
    let max_cue_id = match self.occupancy_state {
      OccupancyState::Settling {
        max_cue_id,
        settling_cue_id: cue_id,
      } => {
        ctx.cancel_cue(cue_id);
        max_cue_id
      }
      OccupancyState::Ready => ctx.cue(MaxSettle, self.config.occupancy_settling_max.once()),
    };

    self.occupancy_state = OccupancyState::Settling {
      max_cue_id,
      settling_cue_id: ctx.cue(VerifyOccupancy, self.config.occupancy_settling_time.once()),
    }
  }

  /// This method gets triggered by a trough switch being hit, but after settling time has elapsed
  /// It should always end with a reset and only emit an event if occupancy did change
  fn verify_occupancy(&mut self, ctx: &SystemContext) {
    // Either cue completes settling; cancel the other so it can't cut short a later settle
    let OccupancyState::Settling {
      settling_cue_id,
      max_cue_id,
    } = self.occupancy_state
    else {
      return;
    };
    ctx.cancel_cue(settling_cue_id);
    ctx.cancel_cue(max_cue_id);
    self.occupancy_state = OccupancyState::Ready;

    // While an eject is verifying, whether the ball has exited isn't known yet. Hold the report until the eject
    // completes so the exit is counted rather than mistaken for (or cancelling out) an entry.
    if !matches!(self.eject_state, EjectState::Verifying { .. }) {
      self.report_occupancy(ctx);
    }
    self.process_queued_ejects(ctx);
  }

  /// Emits the change in occupancy since the last report, separated into entries and exits
  fn report_occupancy(&mut self, ctx: &SystemContext) {
    let current_occupancy = self.read_switch_occupancy(ctx);
    let delta = current_occupancy as i8 - self.last_occupancy as i8;

    // Exits are known from verified ejects; anything beyond that in the net change is an entry. Clamping keeps
    // entered - exited == delta even if a ball vanished without an eject (e.g. a switch fault).
    let entered = (delta + self.exits_since_report as i8).max(0) as u8;
    let exited = (entered as i8 - delta) as u8;

    if entered > 0 || exited > 0 {
      ctx.emit(TroughOccupancyChanged {
        current_occupancy,
        entered,
        exited,
      });
    }

    self.last_occupancy = current_occupancy;
    self.exits_since_report = 0;
  }
}

impl System for TroughSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = *ctx.current_handle();
    self.last_occupancy = self.read_switch_occupancy(ctx);
    self.eject_ready(ctx);
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<VerifyEject>() && matches!(self.eject_state, EjectState::Verifying { .. }) {
      self.verify_eject_fallback(ctx);
    } else if event.is::<VerifyOccupancy>() || event.is::<MaxSettle>() {
      self.verify_occupancy(ctx);
    } else if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      if self.config.trough_switch_names.contains(&e.switch.name) {
        self.queue_occupancy_settling(ctx);
      } else if let Some(q) = &self.config.eject_verification_switches
        && q.matches(&e.switch)
      {
        self.verify_eject_switch(ctx);
      }
    } else if let Some(e) = event.downcast_ref::<SwitchOpened>()
      && self.config.trough_switch_names.contains(&e.switch.name)
    {
      self.queue_occupancy_settling(ctx);
    }
  }
}

// -- States --

#[derive(Debug, Clone, PartialEq, Eq)]
enum OccupancyState {
  Ready,
  Settling {
    settling_cue_id: u64,
    max_cue_id: u64,
  },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EjectState {
  Ready,
  Verifying { target_occupancy: u8, cue_id: u64 },
  Pending,
}

// -- Cues --

#[derive(serde::Serialize, Event)]
struct VerifyEject;

#[derive(serde::Serialize, Event)]
struct VerifyOccupancy;

#[derive(serde::Serialize, Event)]
struct MaxSettle;

// -- Events --

/// Emitted once the trough has settled. Balls that entered and exited within the same settling period are both
/// counted, rather than cancelling out.
#[derive(serde::Serialize, Event)]
pub struct TroughOccupancyChanged {
  pub current_occupancy: u8,
  /// Balls which entered the trough (e.g. drained)
  pub entered: u8,
  /// Balls which exited the trough (verified ejects)
  pub exited: u8,
}

impl TroughOccupancyChanged {
  /// Net change in occupancy
  pub fn delta(&self) -> i8 {
    self.entered as i8 - self.exited as i8
  }
}

#[derive(serde::Serialize, Event)]
pub struct TroughEjecting;

#[derive(serde::Serialize, Event)]
pub struct TroughEjectFailed;

#[cfg(test)]
mod tests {
  use super::*;

  // Trough switches ordered nearest the eject to farthest. Ids: t1..t5 = 0..4, jam = 5, plunge = 6
  const TROUGH: [&str; 5] = ["t1", "t2", "t3", "t4", "t5"];
  const JAM: &str = "jam";
  const PLUNGE: &str = "plunge";

  struct Fixture {
    trough: TroughSystem,
    ctx: TestContext,
    balls: usize,
    jam: bool,
    plunge: bool,
  }

  impl Fixture {
    /// Spawns a trough holding `balls` (filled from the eject end), optionally configured with a jam switch and the
    /// plunge lane as an eject verification switch
    fn spawn(balls: usize, with_jam: bool, with_verify: bool) -> Self {
      let mut fixture = Self::unspawned(balls, with_jam, with_verify);
      fixture.spawn_now();
      fixture
    }

    fn unspawned(balls: usize, with_jam: bool, with_verify: bool) -> Self {
      let mut ctx = TestContext::default();
      for (id, name) in TROUGH.iter().chain([JAM, PLUNGE].iter()).enumerate() {
        ctx.insert_switch(Switch {
          name,
          id,
          ..Default::default()
        });
      }

      let trough = TroughSystem::new(BallManagementConfig {
        trough_switch_names: TROUGH.to_vec(),
        jam_switch_name: with_jam.then_some(JAM),
        eject_coil_name: "eject",
        eject_verification_switches: with_verify.then(|| SwitchQ::name(PLUNGE)),
        ..Default::default()
      });

      let mut fixture = Self {
        trough,
        ctx,
        balls,
        jam: false,
        plunge: false,
      };
      fixture.apply();
      fixture
    }

    fn spawn_now(&mut self) {
      self.trough.on_spawn(&self.ctx.sys_ctx());
      self.events(); // discard anything from setup
    }

    /// Write the fixture's switch state into the context's switch cache
    fn apply(&mut self) {
      let mut states: Vec<SwitchState> = (0..TROUGH.len()).map(|i| state(i < self.balls)).collect();
      states.push(state(self.jam));
      states.push(state(self.plunge));
      self.ctx.update_switch_states(states);
    }

    fn switch(&self, name: &str) -> Switch {
      self.ctx.base.switches.by_name(name).unwrap().clone()
    }

    fn deliver(&mut self, event: &dyn Event) {
      self.trough.on_event(event, &self.ctx.sys_ctx());
    }

    /// Change the trough's ball count, delivering the switch event for the switch that changed
    fn set_balls(&mut self, balls: usize) {
      let previous = self.balls;
      self.balls = balls;
      self.apply();
      if balls > previous {
        let switch = self.switch(TROUGH[balls - 1]);
        self.deliver(&SwitchClosed::new(switch));
      } else if balls < previous {
        let switch = self.switch(TROUGH[balls]);
        self.deliver(&SwitchOpened::new(switch));
      }
    }

    fn set_jam(&mut self, closed: bool) {
      self.jam = closed;
      self.apply();
      let switch = self.switch(JAM);
      if closed {
        self.deliver(&SwitchClosed::new(switch));
      } else {
        self.deliver(&SwitchOpened::new(switch));
      }
    }

    fn close_plunge(&mut self) {
      self.plunge = true;
      self.apply();
      let switch = self.switch(PLUNGE);
      self.deliver(&SwitchClosed::new(switch));
    }

    fn eject(&mut self) {
      self.trough.eject(&self.ctx.svc_ctx());
    }

    fn eject_many(&mut self, count: u8) {
      self.trough.eject_many(count, &self.ctx.svc_ctx());
    }

    /// Simulate the settling cue elapsing
    fn settle(&mut self) {
      self.deliver(&VerifyOccupancy);
    }

    /// Simulate the eject verification cue elapsing
    fn verify_timeout(&mut self) {
      self.deliver(&VerifyEject);
    }

    /// Short names of all events emitted since the last call
    fn events(&mut self) -> Vec<String> {
      self
        .ctx
        .events_emitted()
        .iter()
        .map(|e| e.short_name())
        .collect()
    }

    /// (current_occupancy, entered, exited) of all occupancy events emitted since the last call
    fn occupancy_events(&mut self) -> Vec<(u8, u8, u8)> {
      self
        .ctx
        .events_emitted()
        .iter()
        .filter_map(|e| e.event.downcast_ref::<TroughOccupancyChanged>())
        .map(|e| (e.current_occupancy, e.entered, e.exited))
        .collect()
    }

    fn is_verifying(&self) -> bool {
      matches!(self.trough.eject_state, EjectState::Verifying { .. })
    }

    fn is_settling(&self) -> bool {
      matches!(self.trough.occupancy_state, OccupancyState::Settling { .. })
    }
  }

  fn state(closed: bool) -> SwitchState {
    if closed {
      SwitchState::Closed
    } else {
      SwitchState::Open
    }
  }

  // -- Spawn --

  #[test]
  fn spawn_establishes_occupancy_from_switches() {
    let f = Fixture::spawn(3, false, false);
    assert_eq!(f.trough.occupancy(), 3);
    assert_eq!(f.trough.eject_state, EjectState::Ready);
    assert!(!f.is_settling());
  }

  #[test]
  fn spawn_does_not_count_closed_jam_switch_in_occupancy() {
    let mut f = Fixture::unspawned(2, true, false);
    f.jam = true;
    f.apply();
    f.spawn_now();
    assert_eq!(f.trough.occupancy(), 2);
    assert!(f.trough.is_jammed(&f.ctx.sys_ctx()));
  }

  // -- Occupancy settling --

  #[test]
  fn occupancy_switch_change_while_ready_starts_settling_without_emitting() {
    let mut f = Fixture::spawn(3, false, false);
    f.set_balls(4);
    assert!(f.is_settling());
    assert!(f.events().is_empty());
    assert_eq!(
      f.trough.occupancy(),
      3,
      "occupancy is not updated until settled"
    );
  }

  #[test]
  fn occupancy_settled_after_ball_enters_reports_one_entered() {
    let mut f = Fixture::spawn(3, false, false);
    f.set_balls(4);
    f.settle();
    assert_eq!(f.occupancy_events(), vec![(4, 1, 0)]);
    assert_eq!(f.trough.occupancy(), 4);
    assert!(!f.is_settling());
  }

  #[test]
  fn occupancy_settled_after_ball_leaves_without_eject_reports_one_exited() {
    let mut f = Fixture::spawn(3, false, false);
    f.set_balls(2);
    f.settle();
    assert_eq!(f.occupancy_events(), vec![(2, 0, 1)]);
  }

  #[test]
  fn occupancy_two_balls_entering_while_settling_report_single_event_with_two_entered() {
    let mut f = Fixture::spawn(2, false, false);
    f.set_balls(3);
    f.set_balls(4);
    f.settle();
    assert_eq!(f.occupancy_events(), vec![(4, 2, 0)]);
  }

  #[test]
  fn occupancy_ball_leaving_and_entering_while_settling_with_no_net_change_emits_nothing_and_returns_ready()
   {
    let mut f = Fixture::spawn(3, false, false);
    f.set_balls(2);
    f.set_balls(3);
    f.settle();
    assert!(f.events().is_empty());
    assert!(!f.is_settling());
    assert_eq!(f.trough.occupancy(), 3);
  }

  #[test]
  fn occupancy_change_while_settling_restarts_settle_but_keeps_max_settle() {
    let mut f = Fixture::spawn(3, false, false);
    f.set_balls(4);
    let OccupancyState::Settling {
      settling_cue_id: first_settle,
      max_cue_id: first_max,
    } = f.trough.occupancy_state
    else {
      panic!("expected settling");
    };

    f.set_balls(5);
    let OccupancyState::Settling {
      settling_cue_id: second_settle,
      max_cue_id: second_max,
    } = f.trough.occupancy_state
    else {
      panic!("expected settling");
    };

    assert_ne!(
      first_settle, second_settle,
      "settle cue restarts on every change"
    );
    assert_eq!(first_max, second_max, "max settle cue is not restarted");
  }

  #[test]
  fn occupancy_max_settle_reports_even_while_switches_keep_changing() {
    let mut f = Fixture::spawn(3, false, false);
    f.set_balls(4);
    f.set_balls(3);
    f.set_balls(4);
    f.deliver(&MaxSettle);
    assert_eq!(f.occupancy_events(), vec![(4, 1, 0)]);
    assert!(!f.is_settling());
  }

  #[test]
  fn occupancy_stale_settle_cue_while_ready_is_ignored() {
    let mut f = Fixture::spawn(3, false, false);
    // switch cache changes without a switch event; a stale cue must not report it
    f.balls = 4;
    f.apply();
    f.settle();
    f.deliver(&MaxSettle);
    assert!(f.events().is_empty());
    assert_eq!(f.trough.occupancy(), 3);
  }

  #[test]
  fn occupancy_jam_switch_change_does_not_start_settling() {
    let mut f = Fixture::spawn(3, true, false);
    f.set_jam(true);
    assert!(!f.is_settling());
    assert!(f.events().is_empty());
  }

  // -- Eject requests --

  #[test]
  fn eject_when_ready_and_settled_emits_ejecting_and_verifies_against_one_less_ball() {
    let mut f = Fixture::spawn(3, false, true);
    f.eject();
    assert_eq!(f.events(), vec!["TroughEjecting"]);
    assert!(matches!(
      f.trough.eject_state,
      EjectState::Verifying {
        target_occupancy: 2,
        ..
      }
    ));
  }

  #[test]
  fn eject_when_empty_is_queued_until_a_ball_enters_and_settles() {
    let mut f = Fixture::spawn(0, false, true);
    f.eject();
    assert!(f.events().is_empty());
    assert_eq!(f.trough.eject_queue, 1);
    assert_eq!(f.trough.eject_state, EjectState::Ready);

    f.set_balls(1);
    f.settle();
    assert_eq!(f.events(), vec!["TroughOccupancyChanged", "TroughEjecting"]);
    assert_eq!(f.trough.eject_queue, 0);
    assert!(f.is_verifying());
  }

  #[test]
  fn eject_when_empty_but_jammed_ejects_to_clear_the_jam() {
    let mut f = Fixture::unspawned(0, true, true);
    f.jam = true;
    f.apply();
    f.spawn_now();
    f.eject();
    assert_eq!(f.events(), vec!["TroughEjecting"]);
    assert!(f.is_verifying());
  }

  #[test]
  fn eject_while_settling_is_queued_and_runs_after_settle() {
    let mut f = Fixture::spawn(3, false, true);
    f.set_balls(4);
    f.eject();
    assert!(f.events().is_empty());
    assert_eq!(f.trough.eject_queue, 1);

    f.settle();
    assert_eq!(f.events(), vec!["TroughOccupancyChanged", "TroughEjecting"]);
    assert!(f.is_verifying());
  }

  #[test]
  fn eject_while_verifying_is_queued_and_runs_after_verification() {
    let mut f = Fixture::spawn(3, false, true);
    f.eject();
    f.eject();
    assert_eq!(f.events(), vec!["TroughEjecting"]);
    assert_eq!(f.trough.eject_queue, 1);

    f.set_balls(2); // ejected ball leaves
    f.settle();
    f.close_plunge();
    // the held exit is reported before the queued eject starts
    assert_eq!(f.events(), vec!["TroughOccupancyChanged", "TroughEjecting"]);
    assert_eq!(f.trough.eject_queue, 0);
    assert!(f.is_verifying());
  }

  #[test]
  fn eject_before_spawn_is_queued_and_runs_on_spawn() {
    let mut f = Fixture::unspawned(3, false, true);
    f.eject();
    assert_eq!(f.trough.eject_queue, 1);

    f.trough.on_spawn(&f.ctx.sys_ctx());
    assert_eq!(f.events(), vec!["TroughEjecting"]);
    assert!(f.is_verifying());
  }

  #[test]
  fn eject_many_with_zero_does_nothing() {
    let mut f = Fixture::spawn(3, false, true);
    f.eject_many(0);
    assert!(f.events().is_empty());
    assert_eq!(f.trough.eject_queue, 0);
    assert_eq!(f.trough.eject_state, EjectState::Ready);
  }

  #[test]
  fn eject_many_ejects_one_immediately_and_queues_the_rest() {
    let mut f = Fixture::spawn(3, false, true);
    f.eject_many(3);
    assert_eq!(f.events(), vec!["TroughEjecting"]);
    assert_eq!(f.trough.eject_queue, 2);
    assert!(f.is_verifying());
  }

  // -- Verification by switch --

  #[test]
  fn verify_switch_closed_while_verifying_completes_eject() {
    let mut f = Fixture::spawn(3, false, true);
    f.eject();
    f.close_plunge();
    assert_eq!(f.trough.eject_state, EjectState::Ready);
    assert_eq!(f.trough.eject_retries, 0);
  }

  #[test]
  fn verify_switch_closed_while_ready_is_ignored() {
    let mut f = Fixture::spawn(3, false, true);
    f.close_plunge();
    assert_eq!(f.trough.eject_state, EjectState::Ready);
    assert!(f.events().is_empty());
  }

  #[test]
  fn verify_switch_completes_eject_even_when_a_ball_drained_during_the_eject() {
    let mut f = Fixture::spawn(3, false, true);
    f.eject();
    f.set_balls(2); // ejected ball leaves
    f.set_balls(3); // another ball drains in
    f.close_plunge();
    assert_eq!(f.trough.eject_state, EjectState::Ready);
    assert_eq!(f.trough.eject_retries, 0);
  }

  #[test]
  fn verify_switch_completion_while_settling_defers_next_queued_eject_until_settled() {
    let mut f = Fixture::spawn(3, false, true);
    f.eject_many(2);
    f.events();
    f.set_balls(2); // ejected ball leaves, trough starts settling
    f.close_plunge();
    assert_eq!(f.trough.eject_state, EjectState::Ready);
    assert_eq!(f.trough.eject_queue, 1);
    assert!(f.events().is_empty());

    f.settle();
    assert_eq!(f.events(), vec!["TroughOccupancyChanged", "TroughEjecting"]);
    assert!(f.is_verifying());
  }

  // -- Verification by timeout (fallback) --

  #[test]
  fn verify_timeout_with_occupancy_dropped_completes_eject() {
    let mut f = Fixture::spawn(3, false, false);
    f.eject();
    f.set_balls(2);
    f.settle();
    f.verify_timeout();
    assert_eq!(f.trough.eject_state, EjectState::Ready);
    assert_eq!(f.trough.eject_retries, 0);
  }

  #[test]
  fn verify_timeout_with_occupancy_unchanged_retries_without_emitting_ejecting() {
    let mut f = Fixture::spawn(3, false, false);
    f.eject();
    f.events();
    f.verify_timeout();
    assert!(f.is_verifying());
    assert_eq!(f.trough.eject_retries, 1);
    assert!(f.events().is_empty());
  }

  #[test]
  fn verify_timeout_with_jam_closed_retries_even_though_occupancy_dropped() {
    let mut f = Fixture::spawn(3, true, false);
    f.eject();
    // ball was lifted and stuck on top: its position switch opens and the jam switch closes
    f.set_balls(2);
    f.set_jam(true);
    f.settle();
    f.verify_timeout();
    assert!(f.is_verifying());
    assert_eq!(f.trough.eject_retries, 1);
  }

  #[test]
  fn verify_timeout_while_settling_defers_retry_until_settled() {
    let mut f = Fixture::spawn(3, false, false);
    f.eject();
    f.events();
    f.set_balls(4); // a ball drains right before the timeout
    f.verify_timeout();
    assert_eq!(f.trough.eject_state, EjectState::Ready);
    assert_eq!(f.trough.eject_queue, 1);
    assert_eq!(
      f.trough.eject_retries, 1,
      "deferred retry keeps its retry count"
    );

    f.settle();
    assert!(f.is_verifying());
    assert_eq!(f.trough.eject_retries, 1);
    // the deferred retry goes through eject(), so TroughEjecting is emitted again
    assert_eq!(f.events(), vec!["TroughOccupancyChanged", "TroughEjecting"]);
  }

  #[test]
  fn verify_timeout_at_max_retries_emits_failed_and_returns_to_ready() {
    let mut f = Fixture::spawn(3, false, false);
    f.eject();
    for _ in 0..f.trough.config.eject_retries_max {
      f.verify_timeout();
    }
    assert!(f.is_verifying());
    assert_eq!(f.trough.eject_retries, f.trough.config.eject_retries_max);
    f.events();

    f.verify_timeout();
    assert_eq!(f.events(), vec!["TroughEjectFailed"]);
    assert_eq!(f.trough.eject_state, EjectState::Ready);
    assert_eq!(f.trough.eject_retries, 0);
  }

  #[test]
  fn verify_timeout_at_max_retries_continues_with_next_queued_eject() {
    let mut f = Fixture::spawn(3, false, false);
    f.eject_many(2);
    for _ in 0..f.trough.config.eject_retries_max {
      f.verify_timeout();
    }
    f.events();

    f.verify_timeout();
    assert_eq!(f.events(), vec!["TroughEjectFailed", "TroughEjecting"]);
    assert!(f.is_verifying());
    assert_eq!(f.trough.eject_queue, 0);
    assert_eq!(f.trough.eject_retries, 0);
  }

  #[test]
  fn verify_timeout_while_ready_is_ignored() {
    let mut f = Fixture::spawn(3, false, false);
    f.verify_timeout();
    assert_eq!(f.trough.eject_state, EjectState::Ready);
    assert!(f.events().is_empty());
  }

  #[test]
  fn verify_timeout_without_verification_switch_cannot_distinguish_drain_during_eject_and_retries()
  {
    // Known limitation: without a verification switch, a ball draining during the eject looks like a failed eject
    let mut f = Fixture::spawn(3, false, false);
    f.eject();
    f.set_balls(2); // ejected ball leaves
    f.set_balls(3); // another ball drains in
    f.settle();
    f.verify_timeout();
    assert!(f.is_verifying());
    assert_eq!(f.trough.eject_retries, 1);
  }

  #[test]
  fn successful_eject_after_retries_resets_retry_count() {
    let mut f = Fixture::spawn(3, false, true);
    f.eject();
    f.verify_timeout();
    f.verify_timeout();
    assert_eq!(f.trough.eject_retries, 2);

    f.close_plunge();
    assert_eq!(f.trough.eject_state, EjectState::Ready);
    assert_eq!(f.trough.eject_retries, 0);
  }

  // -- Entries and exits --

  #[test]
  fn occupancy_settled_while_eject_verifying_holds_report_until_eject_verified() {
    let mut f = Fixture::spawn(3, false, true);
    f.eject();
    f.events();
    f.set_balls(2); // ejected ball leaves
    f.settle();
    assert!(f.events().is_empty(), "report is held while the eject is verifying");
    assert_eq!(f.trough.occupancy(), 3);

    f.close_plunge();
    assert_eq!(f.occupancy_events(), vec![(2, 0, 1)]);
    assert_eq!(f.trough.occupancy(), 2);
  }

  #[test]
  fn eject_verified_while_settling_reports_exit_when_settled() {
    let mut f = Fixture::spawn(3, false, true);
    f.eject();
    f.events();
    f.set_balls(2); // ejected ball leaves, trough starts settling
    f.close_plunge();
    assert!(f.events().is_empty(), "nothing reported until settled");

    f.settle();
    assert_eq!(f.occupancy_events(), vec![(2, 0, 1)]);
  }

  #[test]
  fn eject_verified_by_timeout_reports_exit() {
    let mut f = Fixture::spawn(3, false, false);
    f.eject();
    f.events();
    f.set_balls(2);
    f.settle();
    assert!(f.events().is_empty());

    f.verify_timeout();
    assert_eq!(f.occupancy_events(), vec![(2, 0, 1)]);
  }

  #[test]
  fn eject_and_drain_in_same_settle_report_one_entered_and_one_exited() {
    let mut f = Fixture::spawn(3, false, true);
    f.eject();
    f.events();
    f.set_balls(2); // ejected ball leaves
    f.set_balls(3); // another ball drains in before the trough settles
    f.settle();
    f.close_plunge();
    assert_eq!(f.occupancy_events(), vec![(3, 1, 1)]);
  }

  #[test]
  fn eject_retry_does_not_count_failed_attempt_as_exit() {
    let mut f = Fixture::spawn(3, false, true);
    f.eject();
    f.events();
    f.verify_timeout(); // first attempt failed, ball still present
    assert!(f.events().is_empty());

    f.set_balls(2); // retry succeeds
    f.settle();
    f.close_plunge();
    assert_eq!(f.occupancy_events(), vec![(2, 0, 1)]);
  }

  #[test]
  fn eject_failed_after_max_retries_releases_held_report_without_exit() {
    let mut f = Fixture::spawn(3, false, false);
    f.eject();
    f.set_balls(4); // a ball drains during the eject; report held
    f.settle();
    f.events();

    for _ in 0..=f.trough.config.eject_retries_max {
      f.verify_timeout();
    }
    assert_eq!(f.events(), vec!["TroughEjectFailed", "TroughOccupancyChanged"]);
    assert_eq!(f.trough.occupancy(), 4);
  }

  #[test]
  fn verify_switch_without_ball_leaving_reports_phantom_entry_and_exit() {
    // Known limitation: a verification switch hit by a ball that didn't come from the trough (e.g. re-entering the
    // plunge lane) is counted as an exit, so the unchanged occupancy looks like one ball in and one ball out
    let mut f = Fixture::spawn(3, false, true);
    f.eject();
    f.events();
    f.close_plunge();
    assert_eq!(f.occupancy_events(), vec![(3, 1, 1)]);
  }

  #[test]
  fn occupancy_event_delta_is_entered_minus_exited() {
    let event = TroughOccupancyChanged {
      current_occupancy: 3,
      entered: 1,
      exited: 2,
    };
    assert_eq!(event.delta(), -1);
  }
}
