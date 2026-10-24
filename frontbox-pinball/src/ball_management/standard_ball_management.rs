use std::collections::VecDeque;

use frontbox::prelude::*;

use crate::ball_management::trough::TroughSystem;
use crate::ball_management::{
  AutoPlungerFired, AutoPlungerSystem, BallEnteredPlay, BallEnteredPlungeLane,
  BallExitedPlungeLane, BallManagement, BallReenteredPlungeLane, BallsExitedPlay, Launch,
  LaunchMethod, Launcher, MaxBallsInPlayChanged, PlungeLaneSystem, StandardBallManagementConfig,
  TroughEjectFailed, TroughOccupancyChanged,
};

/// Ball management for machines where every ball not in play comes to rest in a single trough, optionally ejecting
/// into a plunge lane (with or without an autoplunger). Set up with `BallManager::standard`.
pub struct StandardBallManagement {
  handle: SystemHandle,
  config: StandardBallManagementConfig,
  max_balls_in_play: u8,
  locked_balls: u8,
  /// Balls requested but not yet ejected from the trough
  queue: VecDeque<Launch>,
  /// The trough is ejecting a ball and hasn't reported it gone yet. This can overlap with the eject state, as the
  /// trough's report can arrive after the ball has reached the plunge lane.
  trough_busy: bool,
  /// Where the ball being ejected is on its way into play. Only one ball at a time.
  eject_state: EjectState,
}

/// Where the ball being ejected is between the trough and play. Only used when configured with a plunge lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EjectState {
  Idle,
  /// Ejected from the trough, on its way to the plunge lane
  EnRoute {
    launch_type: Launch,
    /// Set once the trough has finished ejecting, while waiting for the plunge lane to see the ball
    arrival_cue_id: Option<u64>,
    /// The autoplunger has already been fired once to knock the ball loose
    kicked: bool,
  },
  /// In the plunge lane, from arriving until it is gone for good
  InLane {
    launch_type: Launch,
    /// Requested with `eject`, as opposed to a ball nobody asked for (e.g. one left in the lane at boot)
    claimed: bool,
    launched_by: Option<LaunchMethod>,
  },
}

impl StandardBallManagement {
  pub fn new(config: StandardBallManagementConfig) -> Self {
    StandardBallManagement {
      handle: SystemHandle::default(),
      max_balls_in_play: 0, // set on spawn
      config,
      locked_balls: 0,
      queue: VecDeque::new(),
      trough_busy: false,
      eject_state: EjectState::Idle,
    }
  }

  fn in_play(&self, occupancy: u8) -> u8 {
    self
      .max_balls_in_play
      .saturating_sub(occupancy)
      .saturating_sub(self.locked_balls)
  }

  fn current_in_play(&self, ctx: &SystemContext) -> u8 {
    self.in_play(ctx.expect::<TroughSystem>().occupancy())
  }

  fn max_in_play(&self) -> u8 {
    self.max_balls_in_play.saturating_sub(self.locked_balls)
  }

  /// If somehow the trough reports more balls than we were expecting
  /// attempt to figure out where it came from and account for it
  fn reconcile_appearing_ball(&mut self, delta: u8, ctx: &SystemContext) {
    let mut rem = delta;

    // first reduce it from locked balls
    loop {
      if rem > 0 && self.locked_balls > 0 {
        self.ball_returned_to_play(ctx.into());
        rem -= 1;
      } else {
        break;
      }
    }

    // then increment max balls
    if rem > 0 {
      log::warn!(target: "frontbox::ball_management", "{} unexpected ball(s) entered the trough, increasing max balls in play", rem);
      self.max_balls_in_play += rem;
      ctx.emit(MaxBallsInPlayChanged {
        max_in_play: self.max_in_play(),
        delta: rem as i8,
      });
    }
  }

  fn on_trough_entered(&mut self, current_occupancy: u8, entered: u8, ctx: &SystemContext) {
    // Balls beyond what could be in the trough weren't counted as in play; account for them rather than call them drains
    let excess = current_occupancy
      .saturating_sub(self.max_in_play())
      .min(entered);
    if excess > 0 {
      self.reconcile_appearing_ball(excess, ctx);
    }

    let drained = entered - excess;
    if drained > 0 {
      ctx.emit(BallsExitedPlay {
        count: drained,
        in_play: self.in_play(current_occupancy), // after reconcile
      });
    }
  }

  // -- Launching --

  /// Ejects the next queued ball once the trough is free and the plunge lane (if any) is clear
  fn process_queue(&mut self, ctx: &SystemContext) {
    // A ball nobody asked for that is waiting in the lane for the player serves the next request, rather than a second
    // ball being ejected alongside it. With an autoplunger, such a ball has already been launched.
    if self.config.autoplunger().is_none()
      && let EjectState::InLane {
        launch_type: launch, claimed, ..
      } = &mut self.eject_state
      && !*claimed
      && let Some(next) = self.queue.pop_front()
    {
      *launch = next;
      *claimed = true;
      self.launch_from_lane(ctx);
      return;
    }

    if self.trough_busy || self.eject_state != EjectState::Idle {
      return;
    }
    let Some(launch) = self.queue.pop_front() else {
      return;
    };

    self.trough_busy = true;
    if self.config.plunge_lane().is_some() {
      self.eject_state = EjectState::EnRoute {
        launch_type: launch,
        arrival_cue_id: None,
        kicked: false,
      };
      if let Some(mut lane) = ctx.get::<PlungeLaneSystem>() {
        lane.expect_ball();
      }
    }
    ctx.expect::<TroughSystem>().eject(ctx.into());
  }

  fn on_trough_ejected(&mut self, ctx: &SystemContext) {
    // Only ejects requested here are counted as exits by the trough, but guard against one we didn't ask for
    if !self.trough_busy {
      return;
    }
    self.trough_busy = false;
    if self.config.plunge_lane().is_none() {
      ctx.emit(BallEnteredPlay {
        method: LaunchMethod::Auto,
        in_play: self.current_in_play(ctx),
      });
    } else if matches!(self.eject_state, EjectState::EnRoute { .. }) {
      // The trough is done, so the ball should reach the plunge lane shortly
      self.start_arrival_wait(false, ctx);
    }
    self.process_queue(ctx);
  }

  fn on_trough_eject_failed(&mut self, ctx: &SystemContext) {
    // TODO: re-queue with backoff and report it to the game, rather than dropping the ball (see TODO.md)
    log::warn!(target: "frontbox::ball_management", "Trough eject failed, abandoning the ball");
    self.trough_busy = false;
    self.end_en_route(ctx);
    self.process_queue(ctx);
  }

  /// (Re)start waiting for the en route ball to reach the plunge lane
  fn start_arrival_wait(&mut self, kicked: bool, ctx: &SystemContext) {
    let Some(timeout) = self.config.plunge_lane().map(|lane| lane.arrival_timeout) else {
      return;
    };
    if let EjectState::EnRoute {
      arrival_cue_id,
      kicked: was_kicked,
      ..
    } = &mut self.eject_state
    {
      if let Some(cue_id) = arrival_cue_id.take() {
        ctx.cancel_cue(cue_id);
      }
      *arrival_cue_id = Some(ctx.cue(LaneArrivalTimedOut, timeout.once()));
      *was_kicked = kicked;
    }
  }

  /// Ends the en route stage, if that's where the eject is, returning its launch intent
  fn end_en_route(&mut self, ctx: &SystemContext) -> Option<Launch> {
    let EjectState::EnRoute {
      launch_type: launch,
      arrival_cue_id,
      ..
    } = self.eject_state
    else {
      return None;
    };
    if let Some(cue_id) = arrival_cue_id {
      ctx.cancel_cue(cue_id);
    }
    self.eject_state = EjectState::Idle;
    Some(launch)
  }

  /// The trough ejected a ball, but the plunge lane never saw it arrive (e.g. it stopped short of the lane switch, or
  /// the switch is faulty). First try knocking it loose with the autoplunger, then stop waiting so the queue doesn't
  /// stall. Ball counts come from the trough, so giving up doesn't throw them off.
  fn on_lane_arrival_timed_out(&mut self, ctx: &SystemContext) {
    // Anything other than a waiting en route ball means the cue is stale
    let EjectState::EnRoute {
      arrival_cue_id,
      kicked,
      ..
    } = &mut self.eject_state
    else {
      return;
    };
    if arrival_cue_id.take().is_none() {
      return;
    }
    let kicked = *kicked;

    if !kicked && let Some(mut autoplunger) = ctx.get::<AutoPlungerSystem>() {
      log::warn!(target: "frontbox::ball_management", "Ejected ball never reached the plunge lane, firing the autoplunger to knock it loose");
      autoplunger.launch(ctx.into());
      self.start_arrival_wait(true, ctx);
      return;
    }

    log::warn!(target: "frontbox::ball_management", "Ejected ball never reached the plunge lane, assuming it went into play (is the plunge lane switch working?)");
    self.end_en_route(ctx);
    ctx.emit(BallEnteredPlay {
      method: if kicked {
        LaunchMethod::Auto
      } else {
        LaunchMethod::Manual
      },
      in_play: self.current_in_play(ctx),
    });
    self.process_queue(ctx);
  }

  fn on_ball_entered_lane(&mut self, ctx: &SystemContext) {
    // A ball that wasn't ejected (e.g. one already there at spawn) is launched automatically
    let requested = self.end_en_route(ctx);
    self.eject_state = EjectState::InLane {
      launch_type: requested.unwrap_or(Launch::Auto),
      claimed: requested.is_some(),
      launched_by: None,
    };
    self.launch_from_lane(ctx);
  }

  fn on_ball_reentered_lane(&mut self, ctx: &SystemContext) {
    if let EjectState::InLane { launched_by, .. } = &mut self.eject_state {
      *launched_by = None;
    }
    self.launch_from_lane(ctx);
  }

  /// Carry out the launch intent for the ball in the lane with whatever hardware there is. Without an autoplunger, either intent
  /// waits for the player to plunge.
  fn launch_from_lane(&mut self, ctx: &SystemContext) {
    let EjectState::InLane { launch_type: launch, .. } = self.eject_state else {
      return;
    };
    let Some(mut autoplunger) = ctx.get::<AutoPlungerSystem>() else {
      return;
    };

    match launch {
      Launch::Auto => autoplunger.launch(ctx.into()),
      Launch::Player => autoplunger.arm_launch_button(),
    }
  }

  fn on_ball_exited_lane(&mut self, ctx: &SystemContext) {
    if let EjectState::InLane { launched_by, .. } = self.eject_state {
      self.eject_state = EjectState::Idle;
      if let Some(mut autoplunger) = ctx.get::<AutoPlungerSystem>() {
        autoplunger.disarm_launch_button(ctx.into());
      }
      ctx.emit(BallEnteredPlay {
        method: launched_by.unwrap_or(LaunchMethod::Manual),
        in_play: self.current_in_play(ctx),
      });
    }
    self.process_queue(ctx);
  }
}

impl BallManagement for StandardBallManagement {
  fn eject(&mut self, launch: Launch, ctx: &ServiceContext) {
    self.eject_many(1, launch, ctx);
  }

  fn eject_many(&mut self, count: u8, launch: Launch, ctx: &ServiceContext) {
    for _ in 0..count {
      self.queue.push_back(launch);
    }
    self.process_queue(&ctx.for_system(self.handle));
  }

  fn can_auto_launch(&self) -> bool {
    match &self.config.launcher {
      // balls go straight into play
      Launcher::Direct => true,
      Launcher::PlungeLane(_) => false,
      Launcher::AutoPlunger(..) => true,
    }
  }

  fn balls_in_play(&self, ctx: &ServiceContext) -> u8 {
    self.in_play(ctx.expect::<TroughSystem>(self.handle).occupancy())
  }

  fn establish_ball_count(&mut self, ctx: &ServiceContext) {
    self.max_balls_in_play = ctx.expect::<TroughSystem>(self.handle).occupancy();
    self.locked_balls = 0;
  }

  fn ball_temporarily_removed_from_play(&mut self, ctx: &ServiceContext) {
    let prior = self.locked_balls.clone();
    self.locked_balls = self.locked_balls.saturating_add(1);

    if self.locked_balls > prior {
      ctx.for_system(self.handle).emit(MaxBallsInPlayChanged {
        max_in_play: self.max_in_play(),
        delta: -1,
      });
    }
  }

  fn ball_returned_to_play(&mut self, ctx: &ServiceContext) {
    let prior = self.locked_balls.clone();
    self.locked_balls = self.locked_balls.saturating_sub(1);

    if prior > self.locked_balls {
      ctx.for_system(self.handle).emit(MaxBallsInPlayChanged {
        max_in_play: self.max_in_play(),
        delta: 1,
      });
    }
  }
}

impl System for StandardBallManagement {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = *ctx.current_handle();

    let mut trough_config = self.config.trough.clone();
    if let Some(lane) = self.config.plunge_lane()
      && trough_config.eject_verification_switches.is_none()
    {
      // The ball arriving in the plunge lane is the most direct sign it left the trough
      trough_config.eject_verification_switches = Some(SwitchQ::name(lane.switch_name));
    }

    let trough = TroughSystem::new(trough_config);
    // Until the ball count is established (e.g. at game start), assume the balls in the trough now are all the balls there are
    self.max_balls_in_play = trough.read_switch_occupancy(ctx.into());
    ctx.spawn_system(trough);

    if let Some(autoplunger) = self.config.autoplunger() {
      ctx.spawn_system(AutoPlungerSystem::new(autoplunger));
    }
    if let Some(lane) = self.config.plunge_lane() {
      ctx.spawn_system(PlungeLaneSystem::new(lane));
    }
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(TroughOccupancyChanged {
      current_occupancy,
      entered,
      exited,
    }) = event.downcast_ref::<TroughOccupancyChanged>()
    {
      if *entered > 0 {
        self.on_trough_entered(*current_occupancy, *entered, ctx);
      }
      if *exited > 0 {
        self.on_trough_ejected(ctx);
      }
    } else if event.is::<TroughEjectFailed>() {
      self.on_trough_eject_failed(ctx);
    } else if event.is::<BallEnteredPlungeLane>() {
      self.on_ball_entered_lane(ctx);
    } else if event.is::<BallReenteredPlungeLane>() {
      self.on_ball_reentered_lane(ctx);
    } else if let Some(AutoPlungerFired { method }) = event.downcast_ref::<AutoPlungerFired>()
      && let EjectState::InLane { launched_by, .. } = &mut self.eject_state
    {
      *launched_by = Some(*method);
    } else if event.is::<BallExitedPlungeLane>() {
      self.on_ball_exited_lane(ctx);
    } else if event.is::<LaneArrivalTimedOut>() {
      self.on_lane_arrival_timed_out(ctx);
    }
  }
}

// -- Cues --

#[derive(serde::Serialize, Event)]
struct LaneArrivalTimedOut;

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ball_management::{BallManager, PlungeLaneState};

  // Ids: t1..t5 = 0..4, plunge = 5, launch = 6
  const TROUGH: [&str; 5] = ["t1", "t2", "t3", "t4", "t5"];
  const PLUNGE: &str = "plunge";
  const LAUNCH: &str = "launch";

  struct Fixture {
    system: StandardBallManagement,
    ctx: TestContext,
    switch_states: Vec<bool>,
  }

  impl Fixture {
    /// Spawns ball management with `balls` in a 5 switch trough, alongside a spawned trough for it to query
    fn spawn(balls: usize) -> Self {
      Self::build(balls, BallManager::standard(TROUGH, "eject").config())
    }

    /// As `spawn`, with a plunge lane and optionally an autoplunger (whose launch button is the `launch` switch)
    fn with_lane(balls: usize, autoplunger: bool, launch_button: bool) -> Self {
      let lane = BallManager::standard(TROUGH, "eject").plunge_lane(PLUNGE);
      let config = match (autoplunger, launch_button) {
        (false, _) => lane.config(),
        (true, false) => lane.autoplunger("autoplunger").config(),
        (true, true) => lane
          .autoplunger("autoplunger")
          .launch_button(SwitchQ::name(LAUNCH))
          .config(),
      };
      Self::build(balls, config)
    }

    fn build(balls: usize, config: StandardBallManagementConfig) -> Self {
      let mut ctx = TestContext::default();
      for (id, name) in TROUGH.iter().chain([PLUNGE, LAUNCH].iter()).enumerate() {
        ctx.insert_switch(Switch {
          name,
          id,
          ..Default::default()
        });
      }
      let mut switch_states: Vec<bool> = (0..TROUGH.len()).map(|i| i < balls).collect();
      switch_states.extend([false, false]);
      ctx.update_switch_states(switch_states.iter().map(|closed| state(*closed)).collect());

      // The systems ball management would spawn, spawned here so it can find them
      let mut trough = TroughSystem::new(config.trough.clone());
      trough.on_spawn(&ctx.sys_ctx());
      ctx.insert_system(trough);
      if let Some(autoplunger) = config.autoplunger() {
        let mut autoplunger = AutoPlungerSystem::new(autoplunger);
        autoplunger.on_spawn(&ctx.sys_ctx());
        ctx.insert_system(autoplunger);
      }
      if let Some(lane) = config.plunge_lane() {
        let mut plunge_lane = PlungeLaneSystem::new(lane);
        plunge_lane.on_spawn(&ctx.sys_ctx());
        ctx.insert_system(plunge_lane);
      }

      let mut system = StandardBallManagement::new(config);
      system.on_spawn(&ctx.sys_ctx());

      let mut fixture = Self {
        system,
        ctx,
        switch_states,
      };
      fixture.ctx.events_emitted(); // discard anything from setup
      fixture
    }

    fn deliver(&mut self, event: &dyn Event) {
      self.system.on_event(event, &self.ctx.sys_ctx());
    }

    fn set_switch(&mut self, name: &str, closed: bool) {
      let id = self.ctx.base.switches.by_name(name).unwrap().id;
      self.switch_states[id] = closed;
      let states = self.switch_states.iter().map(|c| state(*c)).collect();
      self.ctx.update_switch_states(states);
    }

    fn eject_many(&mut self, count: u8, launch: Launch) {
      self.system.eject_many(count, launch, &self.ctx.svc_ctx());
    }

    /// Simulate the trough reporting that the ball it was ejecting has left
    fn trough_ejected(&mut self) {
      self.trough_changed(4, 0, 1);
    }

    /// Simulate the plunge lane reporting a ball resting in it
    fn lane_entered(&mut self) {
      self.set_switch(PLUNGE, true);
      self.deliver(&BallEnteredPlungeLane::new(
        PlungeLaneState::ExpectedBallPresent,
      ));
    }

    /// Simulate the plunge lane reporting the ball gone for good
    fn lane_exited(&mut self) {
      self.set_switch(PLUNGE, false);
      self.deliver(&BallExitedPlungeLane);
    }

    fn press_launch_button(&mut self) {
      let switch = self.ctx.base.switches.by_name(LAUNCH).unwrap().clone();
      let ctx = self.ctx.sys_ctx();
      ctx
        .expect::<AutoPlungerSystem>()
        .on_event(&SwitchClosed::new(switch), &ctx);
    }

    fn is_launch_button_armed(&self) -> bool {
      self.ctx.sys_ctx().expect::<AutoPlungerSystem>().is_armed()
    }

    /// Delivers autoplunger fires emitted since the last call to ball management, as the app would, and returns them
    fn forward_autoplunger_fires(&mut self) -> Vec<LaunchMethod> {
      let fired: Vec<LaunchMethod> = self
        .ctx
        .events_emitted()
        .iter()
        .filter_map(|e| e.event.downcast_ref::<AutoPlungerFired>())
        .map(|e| e.method)
        .collect();
      for method in &fired {
        self.deliver(&AutoPlungerFired { method: *method });
      }
      fired
    }

    /// The launch intent of the ball on its way to the plunge lane, if there is one
    fn en_route(&self) -> Option<Launch> {
      match self.system.eject_state {
        EjectState::EnRoute { launch_type: launch, .. } => Some(launch),
        _ => None,
      }
    }

    fn is_waiting_for_arrival(&self) -> bool {
      matches!(
        self.system.eject_state,
        EjectState::EnRoute {
          arrival_cue_id: Some(_),
          ..
        }
      )
    }

    /// Methods of all launch events emitted since the last call
    fn launched_events(&mut self) -> Vec<LaunchMethod> {
      self
        .ctx
        .events_emitted()
        .iter()
        .filter_map(|e| e.event.downcast_ref::<BallEnteredPlay>())
        .map(|e| e.method)
        .collect()
    }

    /// Simulate the trough reporting a settled occupancy change
    fn trough_changed(&mut self, current_occupancy: u8, entered: u8, exited: u8) {
      self.deliver(&TroughOccupancyChanged {
        current_occupancy,
        entered,
        exited,
      });
    }

    fn lock(&mut self) {
      self
        .system
        .ball_temporarily_removed_from_play(&self.ctx.svc_ctx());
    }

    fn unlock(&mut self) {
      self.system.ball_returned_to_play(&self.ctx.svc_ctx());
    }

    fn balls_in_play(&self) -> u8 {
      self.system.balls_in_play(&self.ctx.svc_ctx())
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

    /// (count, in_play) of all drain events emitted since the last call
    fn drained_events(&mut self) -> Vec<(u8, u8)> {
      self
        .ctx
        .events_emitted()
        .iter()
        .filter_map(|e| e.event.downcast_ref::<BallsExitedPlay>())
        .map(|e| (e.count, e.in_play))
        .collect()
    }

    /// (max_in_play, delta) of all max balls in play events emitted since the last call
    fn max_changed_events(&mut self) -> Vec<(u8, i8)> {
      self
        .ctx
        .events_emitted()
        .iter()
        .filter_map(|e| e.event.downcast_ref::<MaxBallsInPlayChanged>())
        .map(|e| (e.max_in_play, e.delta))
        .collect()
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
  fn spawn_establishes_max_from_balls_present() {
    let f = Fixture::spawn(4);
    assert_eq!(f.system.max_balls_in_play, 4);
    assert_eq!(
      f.balls_in_play(),
      0,
      "an empty trough switch is not a ball in play"
    );
  }

  // -- Drains --

  #[test]
  fn exit_only_change_emits_nothing() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    assert!(f.events().is_empty());
  }

  #[test]
  fn last_ball_draining_reports_none_in_play() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    f.trough_changed(5, 1, 0);
    assert_eq!(f.drained_events(), vec![(1, 0)]);
  }

  #[test]
  fn drain_during_multiball_reports_balls_in_play() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(2, 0, 3);
    f.trough_changed(3, 1, 0);
    assert_eq!(f.drained_events(), vec![(1, 2)]);
  }

  #[test]
  fn two_balls_draining_together_report_a_single_event() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(2, 0, 3);
    f.trough_changed(4, 2, 0);
    assert_eq!(f.drained_events(), vec![(2, 1)]);
  }

  #[test]
  fn eject_and_drain_in_same_report_counts_the_drain() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    f.trough_changed(4, 1, 1);
    assert_eq!(f.drained_events(), vec![(1, 1)]);
  }

  // -- Locked balls --

  #[test]
  fn lock_emits_max_changed() {
    let mut f = Fixture::spawn(5);
    f.lock();
    assert_eq!(f.max_changed_events(), vec![(4, -1)]);
  }

  #[test]
  fn unlock_emits_max_changed() {
    let mut f = Fixture::spawn(5);
    f.lock();
    f.events();
    f.unlock();
    assert_eq!(f.max_changed_events(), vec![(5, 1)]);
  }

  #[test]
  fn unlock_with_nothing_locked_emits_nothing() {
    let mut f = Fixture::spawn(5);
    f.unlock();
    assert!(f.events().is_empty());
    assert_eq!(f.system.locked_balls, 0);
  }

  #[test]
  fn locked_ball_is_not_counted_as_in_play() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    f.lock(); // ball in play goes into the lock
    f.trough_changed(3, 0, 1); // replacement ball ejected
    f.trough_changed(4, 1, 0);
    assert_eq!(f.drained_events(), vec![(1, 0)]);
  }

  // -- Unexpected balls --

  #[test]
  fn unexpected_ball_with_ball_locked_is_treated_as_released() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    f.lock();
    f.trough_changed(3, 0, 1);
    f.trough_changed(4, 1, 0); // last ball in play drains
    f.events();

    // the locked ball comes loose without being returned to play, and drains
    f.trough_changed(5, 1, 0);
    assert_eq!(f.max_changed_events(), vec![(5, 1)]);
    assert_eq!(f.system.locked_balls, 0);
    assert_eq!(f.system.max_balls_in_play, 5);
  }

  #[test]
  fn unexpected_ball_with_nothing_locked_raises_max() {
    let mut f = Fixture::spawn(4);
    // a ball that was stuck on the playfield comes home
    f.trough_changed(5, 1, 0);
    assert_eq!(f.max_changed_events(), vec![(5, 1)]);
    assert_eq!(f.system.max_balls_in_play, 5);
  }

  #[test]
  fn unexpected_ball_alongside_drain_reports_only_the_drain() {
    let mut f = Fixture::spawn(4);
    f.trough_changed(3, 0, 1);
    f.events();

    // the ball in play drains along with a previously stuck ball
    f.trough_changed(5, 2, 0);
    let events = f.ctx.events_emitted();
    let drained: Vec<(u8, u8)> = events
      .iter()
      .filter_map(|e| e.event.downcast_ref::<BallsExitedPlay>())
      .map(|e| (e.count, e.in_play))
      .collect();
    let max_changed: Vec<(u8, i8)> = events
      .iter()
      .filter_map(|e| e.event.downcast_ref::<MaxBallsInPlayChanged>())
      .map(|e| (e.max_in_play, e.delta))
      .collect();
    assert_eq!(drained, vec![(1, 0)]);
    assert_eq!(max_changed, vec![(5, 1)]);
  }

  // -- Establishing ball count --

  #[test]
  fn establish_ball_count_resets_max_from_trough_and_clears_locks() {
    let mut f = Fixture::spawn(5);
    f.lock();
    f.system.max_balls_in_play = 6; // drifted during the previous game
    f.events();

    f.system.establish_ball_count(&f.ctx.svc_ctx());
    assert_eq!(f.system.max_balls_in_play, 5);
    assert_eq!(f.system.locked_balls, 0);
    assert_eq!(f.balls_in_play(), 0);
    assert!(f.events().is_empty());
  }

  // -- Launching without a plunge lane --

  #[test]
  fn without_lane_next_ball_waits_for_trough_to_finish_eject() {
    let mut f = Fixture::spawn(5);
    f.eject_many(2, Launch::Auto);
    assert!(f.system.trough_busy);
    assert_eq!(f.system.queue.len(), 1);

    f.trough_ejected();
    assert_eq!(f.launched_events(), vec![LaunchMethod::Auto]);
    assert!(f.system.trough_busy, "second ball is now ejecting");
    assert!(f.system.queue.is_empty());
  }

  #[test]
  fn trough_exit_without_a_requested_eject_is_not_a_launch() {
    let mut f = Fixture::spawn(5);
    f.trough_ejected();
    assert!(f.launched_events().is_empty());
  }

  #[test]
  fn failed_eject_moves_on_to_next_ball() {
    let mut f = Fixture::with_lane(5, false, false);
    f.eject_many(2, Launch::Auto);
    f.deliver(&TroughEjectFailed);
    assert!(f.system.trough_busy, "second ball is now ejecting");
    assert!(f.system.queue.is_empty());
    assert_eq!(f.en_route(), Some(Launch::Auto));
  }

  // -- Launching from a plunge lane --

  #[test]
  fn next_ball_waits_for_plunge_lane_to_clear() {
    let mut f = Fixture::with_lane(5, true, false);
    f.eject_many(2, Launch::Auto);
    f.trough_ejected();
    assert!(
      !f.system.trough_busy,
      "trough is free but the lane isn't clear"
    );
    assert_eq!(f.system.queue.len(), 1);

    f.lane_entered();
    assert_eq!(f.forward_autoplunger_fires(), vec![LaunchMethod::Auto]);
    assert_eq!(f.system.queue.len(), 1);

    f.lane_exited();
    assert_eq!(f.launched_events(), vec![LaunchMethod::Auto]);
    assert!(f.system.trough_busy, "second ball is now ejecting");
    assert!(f.system.queue.is_empty());
  }

  #[test]
  fn player_launch_without_autoplunger_waits_for_manual_plunge() {
    let mut f = Fixture::with_lane(5, false, false);
    f.eject_many(1, Launch::Player);
    f.trough_ejected();
    f.lane_entered();
    f.lane_exited();
    assert_eq!(f.launched_events(), vec![LaunchMethod::Manual]);
  }

  #[test]
  fn auto_launch_without_autoplunger_falls_back_to_player() {
    let mut f = Fixture::with_lane(5, false, false);
    assert!(!f.system.can_auto_launch());
    f.eject_many(1, Launch::Auto);
    f.trough_ejected();
    f.lane_entered();
    f.lane_exited();
    assert_eq!(f.launched_events(), vec![LaunchMethod::Manual]);
  }

  #[test]
  fn player_launch_with_autoplunger_but_no_button_waits_for_manual_plunge() {
    let mut f = Fixture::with_lane(5, true, false);
    f.eject_many(1, Launch::Player);
    f.lane_entered();
    assert!(f.forward_autoplunger_fires().is_empty());
    assert!(!f.is_launch_button_armed());
    f.lane_exited();
    assert_eq!(f.launched_events(), vec![LaunchMethod::Manual]);
  }

  #[test]
  fn player_launch_with_button_launches_on_press() {
    let mut f = Fixture::with_lane(5, true, true);
    f.eject_many(1, Launch::Player);
    f.lane_entered();
    assert!(f.is_launch_button_armed());

    f.press_launch_button();
    assert_eq!(f.forward_autoplunger_fires(), vec![LaunchMethod::Button]);
    assert!(!f.is_launch_button_armed());

    f.lane_exited();
    assert_eq!(f.launched_events(), vec![LaunchMethod::Button]);
  }

  #[test]
  fn player_launch_with_button_can_still_be_plunged_manually() {
    let mut f = Fixture::with_lane(5, true, true);
    f.eject_many(1, Launch::Player);
    f.lane_entered();
    f.lane_exited();
    assert_eq!(f.launched_events(), vec![LaunchMethod::Manual]);
    assert!(
      !f.is_launch_button_armed(),
      "disarmed once the ball is gone"
    );
  }

  #[test]
  fn launch_button_does_nothing_while_ball_is_rolling() {
    let mut f = Fixture::with_lane(5, true, true);
    f.eject_many(1, Launch::Player);
    f.lane_entered();
    f.set_switch(PLUNGE, false); // half plunge: the ball is off the switch but not yet gone

    f.press_launch_button();
    assert!(f.forward_autoplunger_fires().is_empty());
    assert!(f.is_launch_button_armed());
  }

  #[test]
  fn auto_launch_does_not_arm_launch_button() {
    let mut f = Fixture::with_lane(5, true, true);
    f.eject_many(1, Launch::Auto);
    f.lane_entered();
    assert_eq!(f.forward_autoplunger_fires(), vec![LaunchMethod::Auto]);
    assert!(!f.is_launch_button_armed());
  }

  #[test]
  fn unexpected_ball_in_lane_is_launched_automatically() {
    let mut f = Fixture::with_lane(5, true, false);
    f.lane_entered();
    assert_eq!(f.forward_autoplunger_fires(), vec![LaunchMethod::Auto]);
  }

  #[test]
  fn request_takes_unclaimed_ball_waiting_in_lane_instead_of_ejecting_another() {
    let mut f = Fixture::with_lane(5, false, false);
    f.lane_entered(); // e.g. left in the lane at boot
    f.eject_many(1, Launch::Player);
    assert!(!f.system.trough_busy, "no second ball ejected");
    assert!(f.system.queue.is_empty());

    f.lane_exited();
    assert_eq!(f.launched_events(), vec![LaunchMethod::Manual]);
    assert!(!f.system.trough_busy, "still no second ball once it's gone");
  }

  #[test]
  fn with_autoplunger_unclaimed_lane_ball_is_launched_and_request_waits_for_its_own_ball() {
    let mut f = Fixture::with_lane(5, true, false);
    f.lane_entered();
    assert_eq!(f.forward_autoplunger_fires(), vec![LaunchMethod::Auto]);
    f.eject_many(1, Launch::Player);
    assert_eq!(f.system.queue.len(), 1, "waits for the lane to clear");

    f.lane_exited();
    assert!(f.system.trough_busy, "its own ball is now ejecting");
  }

  #[test]
  fn ball_rolling_back_after_autoplunge_is_launched_again() {
    let mut f = Fixture::with_lane(5, true, false);
    f.eject_many(1, Launch::Auto);
    f.lane_entered();
    f.forward_autoplunger_fires();

    f.deliver(&BallReenteredPlungeLane);
    assert_eq!(f.forward_autoplunger_fires(), vec![LaunchMethod::Auto]);
  }

  // -- Ball never reaching the plunge lane --

  #[test]
  fn ball_reaching_lane_before_trough_finishes_starts_no_arrival_wait() {
    let mut f = Fixture::with_lane(5, true, false);
    f.eject_many(1, Launch::Auto);
    f.lane_entered();
    f.trough_ejected();
    assert!(!f.is_waiting_for_arrival());
  }

  #[test]
  fn ball_not_reaching_lane_is_kicked_then_given_up_on() {
    let mut f = Fixture::with_lane(5, true, false);
    f.eject_many(2, Launch::Player);
    f.trough_ejected();
    assert!(f.is_waiting_for_arrival());

    f.deliver(&LaneArrivalTimedOut);
    assert_eq!(f.forward_autoplunger_fires(), vec![LaunchMethod::Auto]);
    assert!(f.en_route().is_some(), "still waiting after the kick");
    assert!(f.launched_events().is_empty());

    f.deliver(&LaneArrivalTimedOut);
    assert_eq!(f.launched_events(), vec![LaunchMethod::Auto]);
    assert!(f.en_route().is_some(), "the second ball is now en route");
    assert!(f.system.trough_busy);
    assert!(f.system.queue.is_empty());
  }

  #[test]
  fn ball_not_reaching_lane_without_autoplunger_is_given_up_on_at_first_timeout() {
    let mut f = Fixture::with_lane(5, false, false);
    f.eject_many(1, Launch::Player);
    f.trough_ejected();

    f.deliver(&LaneArrivalTimedOut);
    assert_eq!(f.launched_events(), vec![LaunchMethod::Manual]);
    assert!(f.en_route().is_none());
    assert!(!f.is_waiting_for_arrival());
  }

  #[test]
  fn ball_knocked_loose_by_kick_continues_normally() {
    let mut f = Fixture::with_lane(5, true, false);
    f.eject_many(1, Launch::Auto);
    f.trough_ejected();
    f.deliver(&LaneArrivalTimedOut);
    f.forward_autoplunger_fires();

    // the kicked ball rolls over the lane switch on its way out
    f.lane_entered();
    assert!(!f.is_waiting_for_arrival(), "wait cancelled");
    f.forward_autoplunger_fires();
    f.lane_exited();
    assert_eq!(f.launched_events(), vec![LaunchMethod::Auto]);

    f.deliver(&LaneArrivalTimedOut); // a stale cue is ignored
    assert!(f.launched_events().is_empty());
  }

  #[test]
  fn can_auto_launch_depends_on_hardware() {
    assert!(
      Fixture::spawn(5).system.can_auto_launch(),
      "straight into play"
    );
    assert!(!Fixture::with_lane(5, false, false).system.can_auto_launch());
    assert!(Fixture::with_lane(5, true, false).system.can_auto_launch());
  }

  // -- Known limitations --

  #[test]
  fn locked_ball_draining_unannounced_during_multiball_undercounts_in_play() {
    // Known limitation: without lock switches, a locked ball that comes loose looks like an ordinary drain until the
    // trough holds more balls than expected
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    f.lock();
    f.trough_changed(2, 0, 2); // 2 balls in play
    f.trough_changed(3, 1, 0); // the locked ball comes loose and drains
    assert_eq!(
      f.drained_events(),
      vec![(1, 1)],
      "2 are actually still in play"
    );
  }
}
