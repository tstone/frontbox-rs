use frontbox_core::prelude::*;

use crate::ball_management::{
  BallEnteredPlay, BallEnteredPlungeLane, BallExitedPlungeLane, BallManager, BallsExitedPlay, BallsHeldChanged,
  MaxBallsInPlayChanged,
};
use crate::cradle::Cradle;

/// How long a flipper is held with no playfield switch hit before the ball is taken as cradled
const DEFAULT_SETTLE: Duration = Duration::from_secs(1);

/// Works out when the player is cradling the ball, into the `Cradle` state, for systems that pause a timer while they
/// do.
///
/// The ball is cradled once one of the `hold_switches` (the flipper buttons) has been closed, with exactly one ball
/// loose and no `playfield_switches` hit, for the settle time (1 second by default). That's counted from whichever
/// came last, the flipper going up or the last hit, so a ball that rolls down an inlane onto a raised flipper is
/// cradled once it comes to rest there. A loose ball is one in play that isn't held (`set_balls_held`) or waiting in
/// the plunge lane.
///
/// It ends when a flipper is released, a playfield switch is hit (the ball left the flipper), or a ball loose
/// becomes more or fewer than one. With two balls loose, a quiet playfield doesn't say where either is, so there's no
/// cradle during a multiball until all but one ball is held or drained.
///
/// ## Setup
/// ```rust,ignore
/// // at the root
/// CradleSystem::new(
///   SwitchQ::tag::<BallActivity>(),
///   SwitchQ::and(SwitchQ::tag::<Flipper>(), SwitchQ::tag::<Button>()),
/// )
/// ```
///
/// The playfield switches mustn't include the flippers' own end-of-stroke switches, as holding a flipper up keeps
/// those closed (see `tags::BallActivity`).
pub struct CradleSystem {
  playfield_switches: SwitchQ,
  hold_switches: SwitchQ,
  settle: Duration,
  settle_cue: Option<u64>,
  /// Mirrors the `Cradle` state, which is set by message, so it's up to date within a handler
  cradled: bool,
}

impl CradleSystem {
  pub fn new(playfield_switches: SwitchQ, hold_switches: SwitchQ) -> Self {
    Self {
      playfield_switches,
      hold_switches,
      settle: DEFAULT_SETTLE,
      settle_cue: None,
      cradled: false,
    }
  }

  /// How long a flipper is held with no playfield switch hit before the ball is taken as cradled
  pub fn settle(mut self, settle: Duration) -> Self {
    self.settle = settle;
    self
  }

  /// True while the player is cradling the ball. Same as `ctx.states.is(Cradle::Cradled)`.
  pub fn is_cradled(&self) -> bool {
    self.cradled
  }

  fn is_flipper_held(&self, ctx: &SystemContext) -> bool {
    ctx
      .switches
      .query_iter(&self.hold_switches)
      .any(|switch| ctx.switches.is_closed_by_id(switch.id).unwrap_or(false))
  }

  /// Balls in play that aren't held or waiting in the plunge lane
  fn balls_loose(&self, ctx: &SystemContext) -> u8 {
    ctx.systems.get::<BallManager>().map_or(0, |ball_manager| {
      let waiting = u8::from(ball_manager.is_ball_in_plunge_lane(ctx.into()));
      ball_manager
        .balls_in_play(ctx.into())
        .saturating_sub(ball_manager.balls_held())
        .saturating_sub(waiting)
    })
  }

  fn could_be_cradled(&self, ctx: &SystemContext) -> bool {
    self.is_flipper_held(ctx) && self.balls_loose(ctx) == 1
  }

  /// End any cradle, and start the settle time over while a flipper is held. The balls loose are checked once it
  /// elapses, not now: they can be briefly off while ball management catches up (a ball reaching the plunge lane
  /// before the trough has settled on it leaving), and no event says when it has.
  fn restart(&mut self, ctx: &SystemContext) {
    self.set_cradled(false, ctx);
    ctx.cues.cancel(self.settle_cue.take());
    if self.is_flipper_held(ctx) {
      self.settle_cue = Some(ctx.cues.start(SettleElapsed, self.settle.once()));
    }
  }

  /// The balls loose may have changed. A cradle goes on only while exactly one is.
  fn on_balls_changed(&mut self, ctx: &SystemContext) {
    if !self.cradled || self.balls_loose(ctx) != 1 {
      self.restart(ctx);
    }
  }

  fn on_settle_elapsed(&mut self, ctx: &SystemContext) {
    self.settle_cue = None;
    if self.could_be_cradled(ctx) {
      self.set_cradled(true, ctx);
    }
  }

  fn set_cradled(&mut self, cradled: bool, ctx: &SystemContext) {
    if self.cradled == cradled {
      return;
    }
    self.cradled = cradled;
    if cradled {
      log::info!(target: "frontbox::cradle", "Ball cradled");
      ctx.states.set(Cradle::Cradled);
    } else {
      log::info!(target: "frontbox::cradle", "Cradle ended");
      ctx.states.set(Cradle::Free);
    }
  }
}

impl System for CradleSystem {
  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(SwitchClosed { switch }) = event.downcast_ref::<SwitchClosed>() {
      if self.playfield_switches.matches(switch) {
        // the ball moved: off the flipper, or not on it yet
        self.restart(ctx);
      } else if self.hold_switches.matches(switch) && !self.cradled {
        // a second flipper going up doesn't end a cradle on the first
        self.restart(ctx);
      }
    } else if let Some(SwitchOpened { switch }) = event.downcast_ref::<SwitchOpened>()
      && self.hold_switches.matches(switch)
    {
      // the ball may have been on that flipper, and rolls off it now
      self.restart(ctx);
    } else if event.is::<BallEnteredPlay>()
      || event.is::<BallsExitedPlay>()
      || event.is::<BallsHeldChanged>()
      || event.is::<MaxBallsInPlayChanged>()
      || event.is::<BallEnteredPlungeLane>()
      || event.is::<BallExitedPlungeLane>()
    {
      self.on_balls_changed(ctx);
    } else if event.is::<SettleElapsed>() {
      self.on_settle_elapsed(ctx);
    }
  }
}

// -- Cues --

#[derive(serde::Serialize, Event)]
struct SettleElapsed;
