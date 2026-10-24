use std::ops::{Deref, DerefMut};

use frontbox::{delegate_system, prelude::*};

use crate::ball_management::StandardBuilder;

/// Ball management is the single owner of the balls: how many there are, where they are, and getting them into play,
/// along with all of the ball handling hardware for its machine (e.g. trough, plunge lane, autoplunger, launch buttons)
/// It spawns whatever child systems it needs to run it. Other systems talk to ball management directly, not child systems.
///
/// ## Responsibilities
/// - **Counting balls.** Knows how many balls are in play, accounting for balls temporarily removed from play (e.g.
///   locked). Counts are derived from what the hardware reports, not from running counters, so that a halted event,
///   failed eject, or missed drain can't make them drift. Balls that appear unexpectedly are reconciled, not reported
///   as drains.
/// - **Reporting drains.** Emits `BallsExitedPlay` with how many balls are still in play. It does not decide what a drain
///   means: ending a turn, saving a ball, or ending a multiball belong to the systems listening. Halting a
///   `BallsExitedPlay` event must not affect ball management's own state.
/// - **Putting balls into play.** Owns the queue of balls requested. Balls are launched one at a time, spaced so the
///   hardware can receive each (e.g. waiting for the plunge lane to clear). Callers state intent with `Launch`, and the
///   implementation adapts it to the hardware, so callers never need to know what the machine has.
/// - **Reporting launches.** Emits `BallEnteredPlay` once a ball has actually gone into play, which can be well after it
///   was requested (e.g. a player waiting to plunge). Anything timed from a ball entering play should start here.
/// - **Establishing the ball count on request.** `establish_ball_count` takes the balls present as all there are. It
///   is called explicitly (e.g. at game start) rather than ball management listening for game events, so it does
///   not depend on game management.
///
/// ## Not responsible for
/// - Game rules: turns, ball save, multiball, extra balls. These are built on ball management's events and calls.
/// - Physically holding or releasing locked balls. Ball management is told about them through
///   `ball_temporarily_removed_from_play` and `ball_returned_to_play`.
///
/// ## Events
/// - `BallsExitedPlay` - Emitted when one or more balls leave play
/// - `BallEnteredPlay` - Emitted when a ball has been launched into play
/// - `MaxBallsInPlayChanged` - Emitted when the number of balls that could be in play changes (e.g. a ball is locked)
pub trait BallManagement: SpawnableSystem {
  /// Puts a ball into play. Balls are queued and launched one at a time.
  fn eject(&mut self, launch: Launch, ctx: &ServiceContext);
  /// Puts `count` balls into play
  fn eject_many(&mut self, count: u8, launch: Launch, ctx: &ServiceContext);
  /// True if `Launch::Auto` can launch a ball without the player
  fn can_auto_launch(&self) -> bool;

  /// Balls currently in play, not counting balls that have been temporarily removed from play
  fn balls_in_play(&self, ctx: &ServiceContext) -> u8;
  /// Takes the balls currently present as all of the balls there are, and clears any temporarily removed from play.
  /// Typically called at the start of a game, so that a ball lost in a previous game isn't waited on forever.
  fn establish_ball_count(&mut self, ctx: &ServiceContext);

  /// When a ball is locked, held, or otherwise should no longer be expected to drain
  fn ball_temporarily_removed_from_play(&mut self, ctx: &ServiceContext);
  /// When a ball is unlocked, released, or otherwise should now be expected to drain
  fn ball_returned_to_play(&mut self, ctx: &ServiceContext);
}

pub struct BallManager {
  inner: Box<dyn BallManagement>,
}

impl BallManager {
  pub fn new(ball_management: impl BallManagement + 'static) -> Self {
    Self {
      inner: Box::new(ball_management),
    }
  }

  /// The standard setup: a single trough, optionally ejecting into a plunge lane, optionally with an autoplunger.
  /// Only the hardware the machine has is given, and only valid setups can be written (e.g. a launch button needs an
  /// autoplunger). Settings can be given at any point in the chain.
  ///
  /// ```rust,ignore
  /// // Ejects straight into play
  /// BallManager::standard(TROUGH_SWITCHES, "trough_eject").build()
  ///
  /// // Manual plunger
  /// BallManager::standard(TROUGH_SWITCHES, "trough_eject")
  ///   .jam_switch("trough_jam")
  ///   .plunge_lane("shooter_lane")
  ///   .build()
  ///
  /// // Manual plunger and autoplunger, with launch buttons (or autoplunger only, with launch buttons)
  /// BallManager::standard(TROUGH_SWITCHES, "trough_eject")
  ///   .jam_switch("trough_jam")
  ///   .plunge_lane("shooter_lane")
  ///   .autoplunger("autoplunger")
  ///   .launch_button(SwitchQ::names(["launch_button", "action_button"]))
  ///   .build()
  /// ```
  ///
  /// * `trough_switch_names` - Trough switches, ordered nearest the eject to farthest
  /// * `eject_coil_name` - The trough eject coil
  pub fn standard(
    trough_switch_names: impl IntoIterator<Item = &'static str>,
    eject_coil_name: &'static str,
  ) -> StandardBuilder {
    StandardBuilder::new(trough_switch_names, eject_coil_name)
  }
}

delegate_system!(BallManager, inner);

impl Deref for BallManager {
  type Target = dyn BallManagement;

  fn deref(&self) -> &Self::Target {
    self.inner.as_ref()
  }
}

impl DerefMut for BallManager {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.inner.as_mut()
  }
}

/// The intent behind putting a ball into play. How it is carried out depends on the machine's hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Launch {
  /// The player launches the ball, with the manual plunger or a launch button (e.g. the start of a turn)
  Player,
  /// The game launches the ball without the player (e.g. ball save, multiball). If the machine has no way to launch
  /// a ball automatically, this falls back to the player launching it.
  Auto,
}

/// How a ball went into play
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum LaunchMethod {
  /// The player plunged it with the manual plunger
  Manual,
  /// The player pressed a launch button, firing the autoplunger
  Button,
  /// The game launched it without the player
  Auto,
}
