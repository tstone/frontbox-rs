use std::collections::VecDeque;

use frontbox_core::prelude::*;

use crate::ball_management::{
  BallEnteredPlay, BallManager, BallSearchFailed, BallSearching, BallsExitedPlay,
  MaxBallsInPlayChanged,
};
use crate::{CoinDoor, configs, metrics};

/// Rounds of `BallSearching` run before giving up
const SEARCH_ROUNDS: u8 = 3;
/// Pause after each round, for a ball knocked loose to show up
const ROUND_PAUSE: Duration = Duration::from_secs(5);
/// Time between coils firing during a search, from one participant or coil to the next, so no two fire at once
const COIL_INTERVAL: Duration = Duration::from_millis(250);

/// Searches for a stuck ball. Once a ball is in play and none of `playfield_switches` (e.g.
/// `SwitchQ::tag::<Playfield>()`) has been hit for `configs::BALL_SEARCH_DELAY`, it runs the `BallSearching` routine,
/// up to 3 rounds with a pause after each. A ball showing up (a playfield switch hit, a drain, or a launch) ends
/// the search. If none has by the end of the last round's pause, it gives up with `BallSearchFailed`, taking one ball
/// out of the count as lost. The delay then starts over, so another ball still in play is searched for in turn.
///
/// Each round runs the routine, then fires the `coils` (e.g. pop bumpers and slingshots, which have nothing to decide)
/// one at a time in driver order. Ending the search cancels the round in progress, so no more coils fire.
///
/// There's no search while a ball waits in the plunge lane for the player, while every ball not in the trough is held
/// (`set_balls_held`), while one of the `hold_switches` is closed (the player cradling the ball on a flipper), or
/// while the coin door is open (`CoinDoor`), as an operator may have a hand on the playfield. Opening the door ends a
/// search, and closing it, releasing a hold switch, or a holder releasing its ball starts the delay over. Add it at
/// the root, where the routine's participants are.
pub struct BallSearchSystem {
  playfield_switches: SwitchQ,
  hold_switches: Option<SwitchQ>,
  coils: Option<DriverQ>,
  /// Coils still to fire in the current round
  coils_to_fire: VecDeque<&'static str>,
  delay_cue: Option<u64>,
  round_cue: Option<u64>,
  /// The `BallSearching` routine of the current round, while it runs
  routine_id: Option<u64>,
  /// Fires the next of `coils_to_fire`
  coil_cue: Option<u64>,
  /// Rounds run in the current search, 0 when not searching
  rounds: u8,
}

impl BallSearchSystem {
  pub fn new(playfield_switches: SwitchQ) -> Self {
    Self {
      playfield_switches,
      hold_switches: None,
      coils: None,
      coils_to_fire: VecDeque::new(),
      delay_cue: None,
      round_cue: None,
      routine_id: None,
      coil_cue: None,
      rounds: 0,
    }
  }

  /// Switches the player holds to keep the ball, such as the flipper buttons. No search starts while any is closed.
  pub fn hold_switches(mut self, switches: SwitchQ) -> Self {
    self.hold_switches = Some(switches);
    self
  }

  /// Coils to fire after the routine in each round
  pub fn coils(mut self, coils: DriverQ) -> Self {
    self.coils = Some(coils);
    self
  }

  /// True while searching, including the pause after each round
  pub fn is_searching(&self) -> bool {
    self.rounds > 0
  }

  fn restart_delay(&mut self, ctx: &SystemContext) {
    if let Some(cue_id) = self.delay_cue.take() {
      ctx.cues.cancel(cue_id);
    }
    let delay = ctx.config.get(&configs::BALL_SEARCH_DELAY);
    self.delay_cue = Some(ctx.cues.start(SearchDelayElapsed, delay.once()));
  }

  fn ball_seen(&mut self, ctx: &SystemContext) {
    if self.is_searching() {
      log::info!(target: "frontbox_core::ball_search", "Ball found, ending the search");
      self.stop_search(ctx);
    }
    self.restart_delay(ctx);
  }

  /// Also stops the round in progress, wherever it's at: in the routine, or firing coils
  fn stop_search(&mut self, ctx: &SystemContext) {
    self.rounds = 0;
    if let Some(cue_id) = self.round_cue.take() {
      ctx.cues.cancel(cue_id);
    }
    if let Some(routine_id) = self.routine_id.take() {
      ctx.routines.cancel(routine_id);
    }
    if let Some(cue_id) = self.coil_cue.take() {
      ctx.cues.cancel(cue_id);
    }
    self.coils_to_fire.clear();
  }

  /// A ball is loose somewhere (in play, and not waiting in the plunge lane), and no operator is at the playfield
  fn has_loose_ball(&self, ctx: &SystemContext) -> bool {
    ctx.states.is(CoinDoor::Closed)
      && ctx
        .systems
        .get::<BallManager>()
        .is_some_and(|ball_manager| {
          ball_manager.balls_in_play(ctx.into()) > 0
            && !ball_manager.is_ball_in_plunge_lane(ctx.into())
        })
  }

  /// One of the `hold_switches` is closed
  fn is_ball_held(&self, ctx: &SystemContext) -> bool {
    self.hold_switches.as_ref().is_some_and(|switches| {
      ctx
        .switches
        .query_iter(switches)
        .any(|switch| ctx.switches.is_closed_by_id(switch.id).unwrap_or(false))
    })
  }

  fn on_delay_elapsed(&mut self, ctx: &SystemContext) {
    self.delay_cue = None;
    // the delay was started over mid-search (e.g. a hold switch released), and starts again once the search ends
    if self.is_searching() {
      return;
    }
    if !self.has_loose_ball(ctx) || self.is_ball_held(ctx) {
      self.restart_delay(ctx);
      return;
    }
    log::info!(target: "frontbox_core::ball_search", "No switch hit with a ball in play, starting a ball search");
    ctx.metrics.inc(&metrics::BALL_SEARCHES);
    self.start_round(ctx);
  }

  fn start_round(&mut self, ctx: &SystemContext) {
    self.rounds += 1;
    self.routine_id = Some(
      ctx
        .routines
        .new(BallSearching { round: self.rounds })
        .interval(COIL_INTERVAL)
        .start(),
    );
  }

  fn on_routine_completed(&mut self, ctx: &SystemContext) {
    self.routine_id = None;
    let mut coils: Vec<&Driver> = self
      .coils
      .as_ref()
      .map_or_else(Vec::new, |coils| coils.query_iter(ctx.into()).collect());
    coils.sort_by_key(|driver| driver.id);
    self.coils_to_fire = coils.into_iter().map(|driver| driver.name).collect();
    self.fire_next_coil_later(ctx);
  }

  fn on_next_coil_due(&mut self, ctx: &SystemContext) {
    self.coil_cue = None;
    if let Some(coil) = self.coils_to_fire.pop_front() {
      ctx.drivers.activate(coil, ActivationMode::Tap);
    }
    self.fire_next_coil_later(ctx);
  }

  /// Wait out the interval before the next coil, or complete the round once there are none left
  fn fire_next_coil_later(&mut self, ctx: &SystemContext) {
    if self.coils_to_fire.is_empty() {
      self.on_round_completed(ctx);
    } else {
      self.coil_cue = Some(ctx.cues.start(NextCoilDue, COIL_INTERVAL.once()));
    }
  }

  fn on_round_completed(&mut self, ctx: &SystemContext) {
    if self.is_searching() {
      self.round_cue = Some(ctx.cues.start(RoundPauseElapsed, ROUND_PAUSE.once()));
    }
  }

  fn on_round_pause_elapsed(&mut self, ctx: &SystemContext) {
    self.round_cue = None;
    if !self.has_loose_ball(ctx) {
      // e.g. the ball was held, or rolled into the plunge lane, without a playfield switch seeing it
      self.stop_search(ctx);
      self.restart_delay(ctx);
    } else if self.rounds < SEARCH_ROUNDS {
      self.start_round(ctx);
    } else {
      self.give_up(ctx);
    }
  }

  fn give_up(&mut self, ctx: &SystemContext) {
    log::warn!(target: "frontbox_core::ball_search", "Ball search gave up after {} rounds", self.rounds);
    let in_play = ctx
      .systems
      .get::<BallManager>()
      .map_or(0, |mut ball_manager| {
        ball_manager.remove_missing_ball(ctx.into());
        ball_manager.balls_in_play(ctx.into())
      });
    ctx.emit(BallSearchFailed {
      rounds: self.rounds,
      in_play,
    });
    self.stop_search(ctx);
    // another ball may still be stuck (e.g. two wedged together)
    self.restart_delay(ctx);
  }
}

impl System for BallSearchSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.restart_delay(ctx);
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(event) = event.downcast_ref::<SwitchClosed>()
      && self.playfield_switches.matches(&event.switch)
    {
      self.ball_seen(ctx);
    } else if let Some(event) = event.downcast_ref::<SwitchOpened>()
      && self
        .hold_switches
        .as_ref()
        .is_some_and(|switches| switches.matches(&event.switch))
    {
      self.restart_delay(ctx);
    } else if event.is::<BallsExitedPlay>() || event.is::<BallEnteredPlay>() {
      self.ball_seen(ctx);
    } else if let Some(MaxBallsInPlayChanged { delta: 1.., .. }) = event.downcast_ref::<MaxBallsInPlayChanged>() {
      // e.g. a holder released its ball, which hasn't had the chance to hit a switch yet
      self.restart_delay(ctx);
    } else if let Some(changed) = event.downcast_ref::<StateChanged<CoinDoor>>() {
      match changed.to {
        CoinDoor::Open => self.stop_search(ctx),
        CoinDoor::Closed => self.restart_delay(ctx),
      }
    } else if event.is::<SearchDelayElapsed>() {
      self.on_delay_elapsed(ctx);
    } else if event.is::<RoundPauseElapsed>() {
      self.on_round_pause_elapsed(ctx);
    } else if event.is::<NextCoilDue>() {
      self.on_next_coil_due(ctx);
    } else if let Some(completed) = event.downcast_ref::<RoutineCompleted<BallSearching>>()
      && completed.completion != RoutineCompletion::Cancelled
    {
      self.on_routine_completed(ctx);
    }
  }
}

// -- Cues --

#[derive(serde::Serialize, Event)]
struct SearchDelayElapsed;

#[derive(serde::Serialize, Event)]
struct RoundPauseElapsed;

#[derive(serde::Serialize, Event)]
struct NextCoilDue;
