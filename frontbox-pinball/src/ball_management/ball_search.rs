use frontbox_core::prelude::*;

use crate::ball_management::{
  BallEnteredPlay, BallManager, BallSearchFailed, BallSearching, BallsExitedPlay,
};
use crate::{CoinDoor, configs, metrics};

/// Rounds of `BallSearching` run before giving up
const SEARCH_ROUNDS: u8 = 3;
/// Pause between rounds, for a ball knocked loose to show up
const ROUND_PAUSE: Duration = Duration::from_secs(5);

/// Searches for a stuck ball. Once a ball is in play and none of `playfield_switches` (e.g.
/// `SwitchQ::tag::<Playfield>()`) has been hit for `configs::BALL_SEARCH_DELAY`, it runs the `BallSearching` routine,
/// up to 3 rounds with a pause between them. A ball showing up (a playfield switch hit, a drain, or a launch) ends
/// the search. After the last round it gives up with `BallSearchFailed`, taking one ball out of the count as lost, and
/// doesn't search again until a ball shows up.
///
/// There's no search while a ball waits in the plunge lane for the player, while every ball not in the trough is held
/// (`set_balls_held`), or while the coin door is open (`CoinDoor`), as an operator may have a hand on the
/// playfield. Opening the door ends a search, and closing it starts the delay over. Add it at the root, where the
/// routine's participants are.
pub struct BallSearchSystem {
  playfield_switches: SwitchQ,
  delay_cue: Option<u64>,
  round_cue: Option<u64>,
  /// Rounds run in the current search, 0 when not searching
  rounds: u8,
  /// A `BallSearching` routine is running. It runs to the end even once the search is over.
  routine_running: bool,
}

impl BallSearchSystem {
  pub fn new(playfield_switches: SwitchQ) -> Self {
    Self {
      playfield_switches,
      delay_cue: None,
      round_cue: None,
      rounds: 0,
      routine_running: false,
    }
  }

  /// True while searching, including the pause between rounds
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

  fn stop_search(&mut self, ctx: &SystemContext) {
    self.rounds = 0;
    if let Some(cue_id) = self.round_cue.take() {
      ctx.cues.cancel(cue_id);
    }
  }

  /// A ball is loose somewhere (in play, and not waiting in the plunge lane), and no operator is at the playfield
  fn has_loose_ball(&self, ctx: &SystemContext) -> bool {
    ctx.states.is(CoinDoor::Closed) && ctx.systems.get::<BallManager>().is_some_and(|ball_manager| {
      ball_manager.balls_in_play(ctx.into()) > 0 && !ball_manager.is_ball_in_plunge_lane(ctx.into())
    })
  }

  fn on_delay_elapsed(&mut self, ctx: &SystemContext) {
    self.delay_cue = None;
    // a round from an ended search is still finishing
    if self.routine_running || !self.has_loose_ball(ctx) {
      self.restart_delay(ctx);
      return;
    }
    log::info!(target: "frontbox_core::ball_search", "No switch hit with a ball in play, starting a ball search");
    ctx.metrics.inc(&metrics::BALL_SEARCHES);
    self.start_round(ctx);
  }

  fn start_round(&mut self, ctx: &SystemContext) {
    self.rounds += 1;
    self.routine_running = true;
    ctx.routines.start(BallSearching { round: self.rounds });
  }

  fn on_round_completed(&mut self, ctx: &SystemContext) {
    self.routine_running = false;
    if !self.is_searching() {
      return;
    }
    if self.rounds < SEARCH_ROUNDS {
      self.round_cue = Some(ctx.cues.start(NextRoundDue, ROUND_PAUSE.once()));
    } else {
      log::warn!(target: "frontbox_core::ball_search", "Ball search gave up after {} rounds", self.rounds);
      let in_play = ctx.systems.get::<BallManager>().map_or(0, |mut ball_manager| {
        ball_manager.remove_missing_ball(ctx.into());
        ball_manager.balls_in_play(ctx.into())
      });
      ctx.emit(BallSearchFailed {
        rounds: self.rounds,
        in_play,
      });
      self.stop_search(ctx);
    }
  }

  fn on_next_round_due(&mut self, ctx: &SystemContext) {
    self.round_cue = None;
    if self.has_loose_ball(ctx) {
      self.start_round(ctx);
    } else {
      // e.g. the ball was held, or rolled into the plunge lane, without a playfield switch seeing it
      self.stop_search(ctx);
      self.restart_delay(ctx);
    }
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
    } else if event.is::<BallsExitedPlay>() || event.is::<BallEnteredPlay>() {
      self.ball_seen(ctx);
    } else if let Some(changed) = event.downcast_ref::<StateChanged<CoinDoor>>() {
      match changed.to {
        CoinDoor::Open => self.stop_search(ctx),
        CoinDoor::Closed => self.restart_delay(ctx),
      }
    } else if event.is::<SearchDelayElapsed>() {
      self.on_delay_elapsed(ctx);
    } else if event.is::<NextRoundDue>() {
      self.on_next_round_due(ctx);
    } else if event.is::<RoutineCompleted<BallSearching>>() {
      self.on_round_completed(ctx);
    }
  }
}

// -- Cues --

#[derive(serde::Serialize, Event)]
struct SearchDelayElapsed;

#[derive(serde::Serialize, Event)]
struct NextRoundDue;
