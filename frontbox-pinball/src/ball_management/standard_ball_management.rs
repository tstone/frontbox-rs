use std::collections::{HashMap, VecDeque};

use frontbox_core::prelude::*;

use crate::ball_management::trough::TroughSystem;
use crate::ball_management::{
  AutoPlungerFired, AutoPlungerSystem, BallEnteredPlay, BallEnteredPlungeLane,
  BallExitedPlungeLane, BallManagement, BallReenteredPlungeLane, BallsExitedPlay, EjectCompleted,
  Launch, LaunchMethod, Launcher, MaxBallsInPlayChanged, PlungeLaneSystem,
  StandardBallManagementConfig, TroughEjectFailed, TroughOccupancyChanged,
};
use crate::{configs, metrics};

/// Ball management for machines where every ball not in play comes to rest in a single trough, optionally ejecting
/// into a plunge lane (with or without an autoplunger). Set up with `BallManager::standard`.
pub struct StandardBallManagement {
  handle: SystemHandle,
  config: StandardBallManagementConfig,
  /// Balls found when the count was last established, plus any that turned up since
  balls_found: u8,
  /// Balls resting in each holder, by the holder's id, as it last reported
  balls_held: HashMap<u64, u8>,
  /// Balls requested but not yet ejected from the trough
  queue: VecDeque<RequestedBall>,
  /// Requests with balls not yet in play or given up on
  requests: Vec<EjectRequest>,
  last_eject_id: u64,
  /// The request of the ball the trough is ejecting, until it reports the ball gone. This can overlap with the eject
  /// state, as the trough's report can arrive after the ball has reached the plunge lane.
  trough_ejecting: Option<u64>,
  /// Where the ball being ejected is on its way into play, or the ball waiting in the plunge lane. Only one ball at a
  /// time.
  eject_state: EjectState,
  /// Set from the autoplunger firing at the ball in the lane until it's checked that the ball left
  launch_verify_cue_id: Option<u64>,
  /// Times the autoplunger has fired again at the ball in the lane without moving it
  launch_retries: u8,
}

/// A ball for a request, and how the request wants it launched
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RequestedBall {
  launch: Launch,
  eject_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EjectRequest {
  id: u64,
  requested: u8,
  /// Balls in play or given up on
  finished: u8,
  launched: u8,
}

/// Where the ball being ejected is between the trough and play. Only used when configured with a plunge lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EjectState {
  Idle,
  /// Ejected from the trough, on its way to the plunge lane
  EnRoute {
    /// The request it serves, or `None` if released by `cancel_ejects`
    ball: Option<RequestedBall>,
    /// Set once the trough has finished ejecting, while waiting for the plunge lane to see the ball
    arrival_cue_id: Option<u64>,
    /// The autoplunger has already been fired once to knock the ball loose
    kicked: bool,
  },
  /// In the plunge lane, from arriving until it is gone for good
  InLane {
    /// The request it serves, or `None` for a ball nobody asked for (e.g. one that rolled back in from the
    /// playfield), which is launched automatically
    ball: Option<RequestedBall>,
    launched_by: Option<LaunchMethod>,
  },
  /// Resting in the plunge lane, not in play, and waiting to serve the next request. Either released by
  /// `cancel_ejects`, or found there without having been counted (e.g. at boot, on a machine without an autoplunger).
  Available,
}

impl StandardBallManagement {
  pub fn new(config: StandardBallManagementConfig) -> Self {
    StandardBallManagement {
      handle: SystemHandle::default(),
      balls_found: 0, // set on spawn
      config,
      balls_held: HashMap::new(),
      queue: VecDeque::new(),
      requests: Vec::new(),
      last_eject_id: 0,
      trough_ejecting: None,
      eject_state: EjectState::Idle,
      launch_verify_cue_id: None,
      launch_retries: 0,
    }
  }

  fn in_play(&self, occupancy: u8) -> u8 {
    self
      .balls_found
      .saturating_sub(occupancy)
      .saturating_sub(self.balls_held())
      .saturating_sub(self.available_balls())
  }

  /// `configs::BALLS_INSTALLED`, where 0 means one per trough switch
  fn balls_installed(&self, ctx: &SystemContext) -> u8 {
    match ctx.config.get(&configs::BALLS_INSTALLED) {
      0 => self.config.trough.trough_switch_names.len() as u8,
      installed => installed,
    }
  }

  /// Balls waiting in the plunge lane to serve the next request, which are neither in the trough nor in play
  fn available_balls(&self) -> u8 {
    (self.eject_state == EjectState::Available) as u8
  }

  fn current_in_play(&self, ctx: &SystemContext) -> u8 {
    self.in_play(ctx.systems.expect::<TroughSystem>().occupancy())
  }

  fn max_in_play(&self) -> u8 {
    self.balls_found.saturating_sub(self.balls_held())
  }

  /// Balls turned up that weren't counted (e.g. one stuck on the playfield since before the count was established),
  /// so they're counted now
  fn reconcile_appearing_ball(&mut self, delta: u8, ctx: &SystemContext) {
    self.count_balls_found(delta, ctx);
    ctx.emit(MaxBallsInPlayChanged {
      max_in_play: self.max_in_play(),
      delta: delta as i8,
    });
  }

  fn count_balls_found(&mut self, delta: u8, ctx: &SystemContext) {
    self.balls_found += delta;
    let installed = self.balls_installed(ctx);
    if self.balls_found > installed {
      log::warn!(target: "frontbox_core::ball_management", "{} ball(s) not counted turned up, now {} found but only {} installed (check the Balls Installed setting)", delta, self.balls_found, installed);
    } else {
      log::info!(target: "frontbox_core::ball_management", "{} ball(s) not counted turned up, now {} of {} found", delta, self.balls_found, installed);
    }
  }

  fn on_trough_entered(&mut self, current_occupancy: u8, entered: u8, ctx: &SystemContext) {
    // Balls beyond what could be in the trough weren't counted as in play; account for them rather than call them drains
    let excess = current_occupancy
      .saturating_sub(self.max_in_play().saturating_sub(self.available_balls()))
      .min(entered);
    if excess > 0 {
      self.reconcile_appearing_ball(excess, ctx);
    }

    let drained = entered - excess;
    if drained > 0 {
      // counted here, where the drain is known, whether or not it is then saved
      ctx.metrics.add(&metrics::BALLS_DRAINED, drained as u64);
      ctx.emit(BallsExitedPlay {
        count: drained,
        in_play: self.in_play(current_occupancy), // after reconcile
      });
    }
  }

  // -- Launching --

  /// A ball from a request has gone into play, or been given up on. The request completes with its last ball.
  fn finish_ball(&mut self, eject_id: Option<u64>, launched: bool, ctx: &SystemContext) {
    let Some(index) = eject_id.and_then(|id| self.requests.iter().position(|r| r.id == id)) else {
      return;
    };
    let request = &mut self.requests[index];
    request.finished += 1;
    if launched {
      request.launched += 1;
    }
    if request.finished >= request.requested {
      let request = self.requests.remove(index);
      ctx.emit(EjectCompleted {
        id: request.id,
        requested: request.requested,
        launched: request.launched,
      });
    }
  }

  /// Ejects the next queued ball once the trough is free and the plunge lane (if any) is clear
  fn process_queue(&mut self, ctx: &SystemContext) {
    // A ball waiting in the lane serves the next request, rather than a second ball being ejected alongside it
    if self.eject_state == EjectState::Available
      && let Some(next) = self.queue.pop_front()
    {
      self.eject_state = EjectState::InLane {
        ball: Some(next),
        launched_by: None,
      };
      self.launch_from_lane(ctx);
      return;
    }

    if self.trough_ejecting.is_some() || self.eject_state != EjectState::Idle {
      return;
    }
    let Some(next) = self.queue.pop_front() else {
      return;
    };

    self.trough_ejecting = Some(next.eject_id);
    if self.config.plunge_lane().is_some() {
      self.eject_state = EjectState::EnRoute {
        ball: Some(next),
        arrival_cue_id: None,
        kicked: false,
      };
    }
    ctx.systems.expect::<TroughSystem>().eject(ctx.into());
  }

  fn on_trough_ejected(&mut self, ctx: &SystemContext) {
    // Only ejects requested here are counted as exits by the trough, but guard against one we didn't ask for
    let Some(eject_id) = self.trough_ejecting.take() else {
      return;
    };
    if self.config.plunge_lane().is_none() {
      ctx.emit(BallEnteredPlay {
        method: LaunchMethod::Auto,
        in_play: self.current_in_play(ctx),
      });
      self.finish_ball(Some(eject_id), true, ctx);
    } else if matches!(self.eject_state, EjectState::EnRoute { .. }) {
      // The trough is done, so the ball should reach the plunge lane shortly
      self.start_arrival_wait(false, ctx);
    }
    self.process_queue(ctx);
  }

  fn on_trough_eject_failed(&mut self, ctx: &SystemContext) {
    // TODO: re-queue with backoff and report it to the game, rather than dropping the ball (see TODO.md)
    log::warn!(target: "frontbox_core::ball_management", "Trough eject failed, abandoning the ball");
    let trough_eject_id = self.trough_ejecting.take();
    let en_route = self.end_en_route(ctx);
    // With a plunge lane, a ball that already reached it isn't abandoned
    let abandoned = match self.config.plunge_lane() {
      Some(_) => en_route.map(|ball| ball.eject_id),
      None => trough_eject_id,
    };
    self.finish_ball(abandoned, false, ctx);
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
      ctx.cues.cancel(arrival_cue_id.take());
      *arrival_cue_id = Some(ctx.cues.start(LaneArrivalTimedOut, timeout.once()));
      *was_kicked = kicked;
    }
  }

  /// Ends the en route stage, if that's where the eject is, returning the request the ball serves
  fn end_en_route(&mut self, ctx: &SystemContext) -> Option<RequestedBall> {
    let EjectState::EnRoute {
      ball,
      arrival_cue_id,
      ..
    } = self.eject_state
    else {
      return None;
    };
    ctx.cues.cancel(arrival_cue_id);
    self.eject_state = EjectState::Idle;
    ball
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

    if !kicked && let Some(mut autoplunger) = ctx.systems.get::<AutoPlungerSystem>() {
      log::warn!(target: "frontbox_core::ball_management", "Ejected ball never reached the plunge lane, firing the autoplunger to knock it loose");
      autoplunger.launch(ctx.into());
      self.start_arrival_wait(true, ctx);
      return;
    }

    log::warn!(target: "frontbox_core::ball_management", "Ejected ball never reached the plunge lane, assuming it went into play (is the plunge lane switch working?)");
    let en_route = self.end_en_route(ctx);
    ctx.emit(BallEnteredPlay {
      method: if kicked {
        LaunchMethod::Auto
      } else {
        LaunchMethod::Manual
      },
      in_play: self.current_in_play(ctx),
    });
    self.finish_ball(en_route.map(|ball| ball.eject_id), true, ctx);
    self.process_queue(ctx);
  }

  fn on_ball_entered_lane(&mut self, ctx: &SystemContext) {
    let was_en_route = matches!(self.eject_state, EjectState::EnRoute { .. });
    let ball = self.end_en_route(ctx);

    if ball.is_none() && was_en_route {
      // Released by `cancel_ejects` on its way here
      self.eject_state = EjectState::Available;
    } else if ball.is_none()
      && self.config.autoplunger().is_none()
      && self.current_in_play(ctx) == 0
    {
      // Nothing was in play, so this ball wasn't counted (e.g. one left in the lane at boot). Count it, and keep it
      // for the next request.
      self.reconcile_appearing_ball(1, ctx);
      self.eject_state = EjectState::Available;
    } else {
      // A ball that wasn't ejected (e.g. one rolled back in from the playfield) is launched automatically
      self.eject_state = EjectState::InLane {
        ball,
        launched_by: None,
      };
      self.launch_from_lane(ctx);
      return;
    }
    self.process_queue(ctx);
  }

  fn on_ball_reentered_lane(&mut self, ctx: &SystemContext) {
    if let EjectState::InLane { launched_by, .. } = &mut self.eject_state {
      *launched_by = None;
    }
    // The last launch did move it
    self.end_launch_verification(ctx);
    self.launch_from_lane(ctx);
  }

  /// Carry out the launch intent for the ball in the lane with whatever hardware there is. Without an autoplunger, either intent
  /// waits for the player to plunge.
  fn launch_from_lane(&mut self, ctx: &SystemContext) {
    let EjectState::InLane { ball, .. } = self.eject_state else {
      return;
    };
    let Some(mut autoplunger) = ctx.systems.get::<AutoPlungerSystem>() else {
      return;
    };

    match ball.map_or(Launch::Auto, |ball| ball.launch) {
      Launch::Auto => autoplunger.launch(ctx.into()),
      Launch::Player => autoplunger.arm_launch_button(),
    }
  }

  fn on_autoplunger_fired(&mut self, method: LaunchMethod, ctx: &SystemContext) {
    let EjectState::InLane { launched_by, .. } = &mut self.eject_state else {
      return;
    };
    *launched_by = Some(method);

    let Some(time) = self
      .config
      .autoplunger()
      .map(|autoplunger| autoplunger.launch_verification_time)
    else {
      return;
    };
    ctx.cues.cancel(self.launch_verify_cue_id.take());
    self.launch_verify_cue_id = Some(ctx.cues.start(VerifyLaunch, time.once()));
  }

  /// The autoplunger fired a while ago. A ball still resting in the lane wasn't moved by it (e.g. a weak coil, or the
  /// ball wasn't seated yet), so an automatic launch is fired again, up to `configs::AUTOPLUNGER_RETRIES` times. Past
  /// that, or for a ball the player launches, the launch button (if any) is armed.
  fn verify_launch(&mut self, ctx: &SystemContext) {
    if self.launch_verify_cue_id.take().is_none() {
      return;
    }
    let EjectState::InLane { ball, .. } = self.eject_state else {
      return;
    };
    // A ball off the switch is either gone, or rolling back and launched again when it re-enters
    if !self.is_lane_ball_resting(ctx) {
      return;
    }
    self.eject_state = EjectState::InLane {
      ball,
      launched_by: None,
    };

    let retries_max = ctx.config.get(&configs::AUTOPLUNGER_RETRIES);
    let launch = ball.map_or(Launch::Auto, |ball| ball.launch);
    if launch == Launch::Auto && self.launch_retries < retries_max {
      self.launch_retries += 1;
      log::info!(target: "frontbox_core::ball_management", "Autoplunger didn't launch the ball, retrying ({}/{})", self.launch_retries, retries_max);
      self.launch_from_lane(ctx);
    } else if let Some(mut autoplunger) = ctx.systems.get::<AutoPlungerSystem>() {
      log::warn!(target: "frontbox_core::ball_management", "Autoplunger didn't launch the ball, leaving it for the player to launch");
      autoplunger.arm_launch_button();
    }
  }

  fn end_launch_verification(&mut self, ctx: &SystemContext) {
    ctx.cues.cancel(self.launch_verify_cue_id.take());
    self.launch_retries = 0;
  }

  fn on_ball_exited_lane(&mut self, ctx: &SystemContext) {
    let (ball, launched_by) = match self.eject_state {
      EjectState::InLane { ball, launched_by } => (ball, launched_by),
      // e.g. plunged by the player while no game was running
      EjectState::Available => (None, None),
      _ => {
        self.process_queue(ctx);
        return;
      }
    };

    self.eject_state = EjectState::Idle;
    self.end_launch_verification(ctx);
    if let Some(mut autoplunger) = ctx.systems.get::<AutoPlungerSystem>() {
      autoplunger.disarm_launch_button(ctx.into());
      autoplunger.cancel_launch(ctx.into());
    }
    ctx.emit(BallEnteredPlay {
      method: launched_by.unwrap_or(LaunchMethod::Manual),
      in_play: self.current_in_play(ctx),
    });
    self.finish_ball(ball.map(|ball| ball.eject_id), true, ctx);
    self.process_queue(ctx);
  }

  /// Whether a ball is resting on the plunge lane switch right now
  fn is_lane_ball_resting(&self, ctx: &SystemContext) -> bool {
    ctx
      .systems.get::<PlungeLaneSystem>()
      .is_some_and(|lane| lane.is_ball_resting(ctx.into()))
  }
}

impl BallManagement for StandardBallManagement {
  fn eject(&mut self, launch: Launch, ctx: &ServiceContext) -> u64 {
    self.eject_many(1, launch, ctx)
  }

  fn eject_many(&mut self, count: u8, launch: Launch, ctx: &ServiceContext) -> u64 {
    let ctx = &ctx.for_system(self.handle);
    self.last_eject_id += 1;
    let eject_id = self.last_eject_id;

    if count == 0 {
      ctx.emit(EjectCompleted {
        id: eject_id,
        requested: 0,
        launched: 0,
      });
      return eject_id;
    }

    self.requests.push(EjectRequest {
      id: eject_id,
      requested: count,
      finished: 0,
      launched: 0,
    });
    for _ in 0..count {
      self.queue.push_back(RequestedBall { launch, eject_id });
    }
    self.process_queue(ctx);
    eject_id
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
    self.in_play(ctx.for_system(self.handle).systems.expect::<TroughSystem>().occupancy())
  }

  fn is_ball_in_plunge_lane(&self, ctx: &ServiceContext) -> bool {
    self.is_lane_ball_resting(&ctx.for_system(self.handle))
  }

  fn cancel_ejects(&mut self, ctx: &ServiceContext) {
    let ctx = &ctx.for_system(self.handle);
    self.queue.clear();

    // An eject the trough hadn't started yet leaves its ball in the trough
    if ctx.systems.expect::<TroughSystem>().cancel_queued_ejects() {
      self.trough_ejecting = None;
      self.end_en_route(ctx);
    }

    // A ball already out of the trough no longer serves a request. One still resting in the lane waits for the next.
    match self.eject_state {
      EjectState::EnRoute { ref mut ball, .. } => *ball = None,
      EjectState::InLane { launched_by, .. } => {
        self.eject_state = if self.is_lane_ball_resting(ctx) {
          EjectState::Available
        } else {
          EjectState::InLane {
            ball: None,
            launched_by,
          }
        };
      }
      EjectState::Idle | EjectState::Available => {}
    }
    self.end_launch_verification(ctx);
    if let Some(mut autoplunger) = ctx.systems.get::<AutoPlungerSystem>() {
      autoplunger.disarm_launch_button(ctx.into());
      autoplunger.cancel_launch(ctx.into());
    }

    for request in std::mem::take(&mut self.requests) {
      ctx.emit(EjectCompleted {
        id: request.id,
        requested: request.requested,
        launched: request.launched,
      });
    }
  }

  fn ejects_pending(&self) -> bool {
    !self.queue.is_empty() || self.trough_ejecting.is_some()
  }

  fn establish_ball_count(&mut self, ctx: &ServiceContext) {
    let ctx = &ctx.for_system(self.handle);
    let occupancy = ctx.systems.expect::<TroughSystem>().occupancy();
    self.balls_found = occupancy + self.available_balls() + self.balls_held();

    let installed = self.balls_installed(ctx);
    if self.balls_found < installed {
      log::warn!(target: "frontbox_core::ball_management", "Only {} of {} installed balls found", self.balls_found, installed);
    }
  }

  fn balls_missing(&self, ctx: &ServiceContext) -> u8 {
    self
      .balls_installed(&ctx.for_system(self.handle))
      .saturating_sub(self.balls_found)
  }

  fn remove_missing_ball(&mut self, ctx: &ServiceContext) {
    let ctx = &ctx.for_system(self.handle);
    if self.current_in_play(ctx) == 0 {
      return;
    }
    self.balls_found -= 1;
    log::warn!(target: "frontbox_core::ball_management", "A ball is lost, playing on with {} of {} installed", self.balls_found, self.balls_installed(ctx));
    ctx.metrics.inc(&metrics::BALLS_LOST);
    ctx.emit(MaxBallsInPlayChanged {
      max_in_play: self.max_in_play(),
      delta: -1,
    });
  }

  fn set_balls_held(&mut self, holder: u64, count: u8, ctx: &ServiceContext) {
    let ctx = &ctx.for_system(self.handle);
    let before = self.balls_held.get(&holder).copied().unwrap_or(0);
    if count == before {
      return;
    }

    // More balls held than were in play weren't counted (e.g. one sitting in a scoop at boot)
    let uncounted = count
      .saturating_sub(before)
      .saturating_sub(self.current_in_play(ctx));
    if uncounted > 0 {
      self.count_balls_found(uncounted, ctx);
    }

    let max_before = self.max_in_play();
    if count == 0 {
      self.balls_held.remove(&holder);
    } else {
      self.balls_held.insert(holder, count);
    }
    let max_after = self.max_in_play();
    let delta = max_after as i8 - max_before as i8 + uncounted as i8;
    if delta != 0 {
      ctx.emit(MaxBallsInPlayChanged {
        max_in_play: max_after,
        delta,
      });
    }
  }

  fn balls_held(&self) -> u8 {
    self.balls_held.values().sum()
  }
}

impl System for StandardBallManagement {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = ctx.handle();

    let mut trough_config = self.config.trough.clone();
    trough_config.check_switch_names(ctx.into());
    if let Some(lane) = self.config.plunge_lane()
      && trough_config.eject_verification_switches.is_none()
    {
      // The ball arriving in the plunge lane is the most direct sign it left the trough
      trough_config.eject_verification_switches = Some(SwitchQ::name(lane.switch_name));
    }

    let trough = TroughSystem::new(trough_config);
    // Until the ball count is established (e.g. at game start), assume the balls in the trough now are all the balls there are
    self.balls_found = trough.read_switch_occupancy(ctx.into());
    ctx.systems.spawn(trough);

    if let Some(autoplunger) = self.config.autoplunger() {
      ctx.systems.spawn(AutoPlungerSystem::new(autoplunger));
    }
    if let Some(lane) = self.config.plunge_lane() {
      ctx.systems.spawn(PlungeLaneSystem::new(lane));
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
    } else if let Some(AutoPlungerFired { method }) = event.downcast_ref::<AutoPlungerFired>() {
      self.on_autoplunger_fired(*method, ctx);
    } else if event.is::<BallExitedPlungeLane>() {
      self.on_ball_exited_lane(ctx);
    } else if event.is::<LaneArrivalTimedOut>() {
      self.on_lane_arrival_timed_out(ctx);
    } else if event.is::<VerifyLaunch>() {
      self.verify_launch(ctx);
    }
  }
}

// -- Cues --

#[derive(serde::Serialize, Event)]
struct LaneArrivalTimedOut;

#[derive(serde::Serialize, Event)]
struct VerifyLaunch;

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ball_management::BallManager;

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

    fn eject_many(&mut self, count: u8, launch: Launch) -> u64 {
      self.system.eject_many(count, launch, &self.ctx.svc_ctx())
    }

    /// (id, requested, launched) of all eject completed events emitted since the last call
    fn completed_events(&mut self) -> Vec<(u64, u8, u8)> {
      self
        .ctx
        .events_emitted()
        .iter()
        .filter_map(|e| e.event.downcast_ref::<EjectCompleted>())
        .map(|e| (e.id, e.requested, e.launched))
        .collect()
    }

    /// Simulate the trough reporting that the ball it was ejecting has left
    fn trough_ejected(&mut self) {
      self.trough_changed(4, 0, 1);
    }

    /// Simulate the plunge lane reporting a ball resting in it
    fn lane_entered(&mut self) {
      self.set_switch(PLUNGE, true);
      self.deliver(&BallEnteredPlungeLane);
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
        .systems.expect::<AutoPlungerSystem>()
        .on_event(&SwitchClosed::new(switch), &ctx);
    }

    fn is_launch_button_armed(&self) -> bool {
      self.ctx.sys_ctx().systems.expect::<AutoPlungerSystem>().is_armed()
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
        EjectState::EnRoute {
          ball: Some(ball), ..
        } => Some(ball.launch),
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
      self
        .ctx
        .sys_ctx()
        .systems.expect::<TroughSystem>()
        .set_occupancy(current_occupancy);
      self.deliver(&TroughOccupancyChanged {
        current_occupancy,
        entered,
        exited,
      });
    }

    /// Simulate `holder` reporting how many balls are resting in it
    fn hold(&mut self, holder: u64, count: u8) {
      self.system.set_balls_held(holder, count, &self.ctx.svc_ctx());
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
  fn spawn_counts_the_balls_in_the_trough() {
    let f = Fixture::spawn(4);
    assert_eq!(f.system.balls_found, 4);
    assert_eq!(
      f.balls_in_play(),
      0,
      "an empty trough switch is not a ball in play"
    );
  }

  #[test]
  #[should_panic(expected = "switches that don't exist: [\"t9\", \"jam\"]")]
  fn spawn_panics_naming_trough_switches_that_dont_exist() {
    let config = BallManager::standard(["t1", "t9"], "eject")
      .jam_switch("jam")
      .config();
    Fixture::build(1, config);
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
  fn drains_are_counted_by_ball() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(2, 0, 3);
    f.trough_changed(4, 2, 0);
    let counts: Vec<_> = f
      .ctx
      .base
      .metrics
      .take_batch()
      .counts
      .iter()
      .map(|delta| (delta.metric, delta.count))
      .collect();
    assert_eq!(counts, vec![("balls_drained", 2)]);
  }

  #[test]
  fn eject_and_drain_in_same_report_counts_the_drain() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    f.trough_changed(4, 1, 1);
    assert_eq!(f.drained_events(), vec![(1, 1)]);
  }

  // -- Held balls --

  #[test]
  fn holding_a_ball_in_play_takes_it_out_of_play() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    f.hold(1, 1);
    assert_eq!(f.max_changed_events(), vec![(4, -1)]);
    assert_eq!(f.balls_in_play(), 0);
  }

  #[test]
  fn releasing_a_held_ball_puts_it_back_in_play() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    f.hold(1, 1);
    f.events();
    f.hold(1, 0);
    assert_eq!(f.max_changed_events(), vec![(5, 1)]);
    assert_eq!(f.balls_in_play(), 1);
  }

  #[test]
  fn reporting_an_unchanged_hold_emits_nothing() {
    let mut f = Fixture::spawn(5);
    f.hold(1, 0);
    assert!(f.events().is_empty());
  }

  #[test]
  fn holders_are_counted_separately() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(3, 0, 2);
    f.hold(1, 1);
    f.hold(2, 1);
    f.hold(1, 0);
    assert_eq!(f.system.balls_held(), 1);
    assert_eq!(f.balls_in_play(), 1);
  }

  #[test]
  fn held_ball_is_not_counted_as_in_play() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    f.hold(1, 1); // ball in play goes into the lock
    f.trough_changed(3, 0, 1); // replacement ball ejected
    f.trough_changed(4, 1, 0);
    assert_eq!(f.drained_events(), vec![(1, 0)]);
  }

  #[test]
  fn ball_released_from_a_holder_during_multiball_is_counted_as_in_play() {
    let mut f = Fixture::spawn(5);
    f.trough_changed(4, 0, 1);
    f.hold(1, 1);
    f.trough_changed(2, 0, 2); // 2 balls in play
    f.hold(1, 0); // the held ball is kicked out
    f.trough_changed(3, 1, 0);
    assert_eq!(f.drained_events(), vec![(1, 2)]);
  }

  #[test]
  fn held_ball_that_was_never_in_play_is_counted() {
    // e.g. one sitting in a scoop at boot
    let mut f = Fixture::spawn(4);
    f.hold(1, 1);
    assert_eq!(f.system.balls_found, 5);
    assert_eq!(f.balls_in_play(), 0);
    assert!(f.max_changed_events().is_empty(), "nothing more could be in play");

    // once kicked out, it's in play and its drain is counted
    f.hold(1, 0);
    assert_eq!(f.balls_in_play(), 1);
    f.trough_changed(5, 1, 0);
    assert_eq!(f.drained_events(), vec![(1, 0)]);
  }

  // -- Unexpected balls --

  #[test]
  fn unexpected_ball_is_counted() {
    let mut f = Fixture::spawn(4);
    // a ball that was stuck on the playfield comes home
    f.trough_changed(5, 1, 0);
    assert_eq!(f.max_changed_events(), vec![(5, 1)]);
    assert_eq!(f.system.balls_found, 5);
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
  fn establish_ball_count_counts_the_trough_and_holders() {
    let mut f = Fixture::spawn(4);
    f.hold(1, 1);
    f.system.balls_found = 7; // drifted during the previous game
    f.events();

    f.system.establish_ball_count(&f.ctx.svc_ctx());
    assert_eq!(f.system.balls_found, 5);
    assert_eq!(f.balls_in_play(), 0);
    assert!(f.events().is_empty());
  }

  #[test]
  fn balls_installed_defaults_to_one_per_trough_switch() {
    let mut f = Fixture::spawn(3);
    f.system.establish_ball_count(&f.ctx.svc_ctx());
    assert_eq!(f.system.balls_missing(&f.ctx.svc_ctx()), 2);
  }

  #[test]
  fn balls_missing_follows_the_balls_installed_setting() {
    let mut f = Fixture::spawn(3);
    f.ctx.svc_ctx().config.set(&configs::BALLS_INSTALLED, 4);
    f.system.establish_ball_count(&f.ctx.svc_ctx());
    assert_eq!(f.system.balls_missing(&f.ctx.svc_ctx()), 1);

    // the missing ball turns up
    f.trough_changed(4, 1, 0);
    assert_eq!(f.system.balls_missing(&f.ctx.svc_ctx()), 0);
  }

  // -- Launching without a plunge lane --

  #[test]
  fn without_lane_next_ball_waits_for_trough_to_finish_eject() {
    let mut f = Fixture::spawn(5);
    f.eject_many(2, Launch::Auto);
    assert!(f.system.trough_ejecting.is_some());
    assert_eq!(f.system.queue.len(), 1);

    f.trough_ejected();
    assert_eq!(f.launched_events(), vec![LaunchMethod::Auto]);
    assert!(
      f.system.trough_ejecting.is_some(),
      "second ball is now ejecting"
    );
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
    assert!(
      f.system.trough_ejecting.is_some(),
      "second ball is now ejecting"
    );
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
      !f.system.trough_ejecting.is_some(),
      "trough is free but the lane isn't clear"
    );
    assert_eq!(f.system.queue.len(), 1);

    f.lane_entered();
    assert_eq!(f.forward_autoplunger_fires(), vec![LaunchMethod::Auto]);
    assert_eq!(f.system.queue.len(), 1);

    f.lane_exited();
    assert_eq!(f.launched_events(), vec![LaunchMethod::Auto]);
    assert!(
      f.system.trough_ejecting.is_some(),
      "second ball is now ejecting"
    );
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
    assert!(
      !f.system.trough_ejecting.is_some(),
      "no second ball ejected"
    );
    assert!(f.system.queue.is_empty());

    f.lane_exited();
    assert_eq!(f.launched_events(), vec![LaunchMethod::Manual]);
    assert!(
      !f.system.trough_ejecting.is_some(),
      "still no second ball once it's gone"
    );
  }

  #[test]
  fn with_autoplunger_unclaimed_lane_ball_is_launched_and_request_waits_for_its_own_ball() {
    let mut f = Fixture::with_lane(5, true, false);
    f.lane_entered();
    assert_eq!(f.forward_autoplunger_fires(), vec![LaunchMethod::Auto]);
    f.eject_many(1, Launch::Player);
    assert_eq!(f.system.queue.len(), 1, "waits for the lane to clear");

    f.lane_exited();
    assert!(
      f.system.trough_ejecting.is_some(),
      "its own ball is now ejecting"
    );
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

  // -- Autoplunger not moving the ball --

  #[test]
  fn auto_launch_that_doesnt_move_the_ball_is_fired_again() {
    let mut f = Fixture::with_lane(5, true, false);
    f.eject_many(1, Launch::Auto);
    f.lane_entered();
    f.forward_autoplunger_fires();

    f.deliver(&VerifyLaunch);
    assert_eq!(f.forward_autoplunger_fires(), vec![LaunchMethod::Auto]);
  }

  #[test]
  fn auto_launch_that_moved_the_ball_is_not_fired_again() {
    let mut f = Fixture::with_lane(5, true, false);
    f.eject_many(1, Launch::Auto);
    f.lane_entered();
    f.forward_autoplunger_fires();
    f.set_switch(PLUNGE, false);

    f.deliver(&VerifyLaunch);
    assert!(f.forward_autoplunger_fires().is_empty());
  }

  #[test]
  fn auto_launch_out_of_retries_arms_the_launch_button() {
    let mut f = Fixture::with_lane(5, true, true);
    let retries_max = f.ctx.svc_ctx().config.get(&configs::AUTOPLUNGER_RETRIES);
    f.eject_many(1, Launch::Auto);
    f.lane_entered();
    for _ in 0..retries_max {
      f.forward_autoplunger_fires();
      f.deliver(&VerifyLaunch);
    }
    assert_eq!(f.forward_autoplunger_fires(), vec![LaunchMethod::Auto]);
    assert!(!f.is_launch_button_armed());

    f.deliver(&VerifyLaunch);
    assert!(f.forward_autoplunger_fires().is_empty());
    assert!(f.is_launch_button_armed());
  }

  #[test]
  fn button_launch_that_doesnt_move_the_ball_rearms_the_button() {
    let mut f = Fixture::with_lane(5, true, true);
    f.eject_many(1, Launch::Player);
    f.lane_entered();
    f.press_launch_button();
    f.forward_autoplunger_fires();

    f.deliver(&VerifyLaunch);
    assert!(f.forward_autoplunger_fires().is_empty());
    assert!(f.is_launch_button_armed());
  }

  #[test]
  fn cancel_stops_launching_the_ball_resting_in_the_lane() {
    let mut f = Fixture::with_lane(5, true, false);
    f.eject_many(1, Launch::Auto);
    f.lane_entered();
    f.forward_autoplunger_fires();
    f.system.cancel_ejects(&f.ctx.svc_ctx());

    f.deliver(&VerifyLaunch);
    assert!(f.forward_autoplunger_fires().is_empty());
    assert_eq!(f.system.eject_state, EjectState::Available);
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
    assert!(f.system.trough_ejecting.is_some());
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

  // -- Eject completion --

  #[test]
  fn without_lane_request_completes_once_its_last_ball_is_ejected() {
    let mut f = Fixture::spawn(5);
    let id = f.eject_many(2, Launch::Auto);
    f.trough_ejected();
    assert!(f.completed_events().is_empty());

    f.trough_ejected();
    assert_eq!(f.completed_events(), vec![(id, 2, 2)]);
  }

  #[test]
  fn with_lane_request_completes_once_its_last_ball_leaves_the_lane() {
    let mut f = Fixture::with_lane(5, true, false);
    let id = f.eject_many(2, Launch::Auto);
    f.trough_ejected();
    f.lane_entered();
    f.lane_exited();
    assert!(f.completed_events().is_empty());

    f.trough_ejected();
    f.lane_entered();
    f.lane_exited();
    assert_eq!(f.completed_events(), vec![(id, 2, 2)]);
  }

  #[test]
  fn requests_complete_separately() {
    let mut f = Fixture::spawn(5);
    let first = f.eject_many(1, Launch::Auto);
    let second = f.eject_many(2, Launch::Auto);
    assert_ne!(first, second);

    f.trough_ejected();
    assert_eq!(f.completed_events(), vec![(first, 1, 1)]);
    f.trough_ejected();
    f.trough_ejected();
    assert_eq!(f.completed_events(), vec![(second, 2, 2)]);
  }

  #[test]
  fn failed_eject_still_completes_the_request() {
    let mut f = Fixture::with_lane(5, true, false);
    let id = f.eject_many(2, Launch::Auto);
    f.deliver(&TroughEjectFailed);
    f.trough_ejected();
    f.lane_entered();
    f.lane_exited();
    assert_eq!(f.completed_events(), vec![(id, 2, 1)]);
  }

  #[test]
  fn ball_given_up_on_reaching_the_lane_counts_as_launched() {
    let mut f = Fixture::with_lane(5, false, false);
    let id = f.eject_many(1, Launch::Player);
    f.trough_ejected();
    f.deliver(&LaneArrivalTimedOut);
    assert_eq!(f.completed_events(), vec![(id, 1, 1)]);
  }

  #[test]
  fn unclaimed_lane_ball_completes_the_request_it_serves() {
    let mut f = Fixture::with_lane(5, false, false);
    f.lane_entered();
    let id = f.eject_many(1, Launch::Player);
    f.lane_exited();
    assert_eq!(f.completed_events(), vec![(id, 1, 1)]);
  }

  #[test]
  fn ball_nobody_asked_for_completes_nothing() {
    let mut f = Fixture::with_lane(5, true, false);
    f.lane_entered();
    f.lane_exited();
    assert!(f.completed_events().is_empty());
  }

  // -- Available lane ball --

  #[test]
  fn ball_left_in_lane_at_boot_is_counted_and_drains_as_the_served_ball() {
    let mut f = Fixture::with_lane(4, false, false);
    f.lane_entered();
    assert_eq!(f.system.eject_state, EjectState::Available);
    assert_eq!(f.balls_in_play(), 0, "waiting for a request, not in play");

    f.system.establish_ball_count(&f.ctx.svc_ctx());
    assert_eq!(f.system.balls_found, 5, "trough and lane");

    f.eject_many(1, Launch::Player);
    assert!(f.system.trough_ejecting.is_none(), "served by the lane ball");
    assert_eq!(f.balls_in_play(), 1);

    f.lane_exited();
    f.trough_changed(5, 1, 0);
    assert_eq!(f.drained_events(), vec![(1, 0)]);
  }

  #[test]
  fn ball_rolling_back_into_lane_while_in_play_stays_in_play() {
    let mut f = Fixture::with_lane(4, false, false);
    f.system.balls_found = 5; // one ball on the playfield
    f.lane_entered();
    assert_eq!(
      f.system.eject_state,
      EjectState::InLane {
        ball: None,
        launched_by: None
      }
    );
    assert_eq!(f.balls_in_play(), 1);
    assert!(f.max_changed_events().is_empty(), "already counted");
  }

  #[test]
  fn cancel_keeps_resting_lane_ball_for_the_next_request() {
    let mut f = Fixture::with_lane(5, true, true);
    let first = f.eject_many(1, Launch::Player);
    f.trough_ejected();
    f.lane_entered();
    assert!(f.is_launch_button_armed());
    assert_eq!(f.system.in_play(4), 1);

    f.system.cancel_ejects(&f.ctx.svc_ctx());
    assert_eq!(f.system.eject_state, EjectState::Available);
    assert_eq!(f.system.in_play(4), 0, "no longer in play");
    assert!(!f.is_launch_button_armed());
    assert_eq!(f.completed_events(), vec![(first, 1, 0)]);

    let second = f.eject_many(1, Launch::Player);
    assert!(f.system.trough_ejecting.is_none(), "no second ball ejected");
    assert!(f.is_launch_button_armed());
    f.press_launch_button();
    f.forward_autoplunger_fires();
    f.lane_exited();
    assert_eq!(f.completed_events(), vec![(second, 1, 1)]);
  }

  #[test]
  fn cancel_releases_ball_on_its_way_to_the_lane_without_launching_it() {
    let mut f = Fixture::with_lane(5, true, false);
    f.eject_many(1, Launch::Auto);
    f.trough_ejected();
    f.system.cancel_ejects(&f.ctx.svc_ctx());

    f.lane_entered();
    assert!(f.forward_autoplunger_fires().is_empty());
    assert_eq!(f.system.eject_state, EjectState::Available);
  }

  #[test]
  fn cancel_drops_queued_balls_and_completes_their_request() {
    let mut f = Fixture::spawn(5);
    let id = f.eject_many(3, Launch::Auto);
    f.system.cancel_ejects(&f.ctx.svc_ctx());
    assert!(f.system.queue.is_empty());
    assert_eq!(f.completed_events(), vec![(id, 3, 0)]);

    // the eject already under way still finishes, without a second completion or another eject
    f.trough_ejected();
    assert_eq!(f.launched_events(), vec![LaunchMethod::Auto]);
    assert!(f.system.trough_ejecting.is_none());
  }

  #[test]
  fn cancel_drops_eject_waiting_for_an_empty_trough() {
    let mut f = Fixture::spawn(0);
    let id = f.eject_many(1, Launch::Auto);
    assert!(f.system.trough_ejecting.is_some());

    f.system.cancel_ejects(&f.ctx.svc_ctx());
    assert!(f.system.trough_ejecting.is_none(), "doesn't block the next request");
    assert_eq!(f.completed_events(), vec![(id, 1, 0)]);
  }

  #[test]
  fn request_for_no_balls_completes_immediately() {
    let mut f = Fixture::spawn(5);
    let id = f.eject_many(0, Launch::Auto);
    assert_eq!(f.completed_events(), vec![(id, 0, 0)]);
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
}
