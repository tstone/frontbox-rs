use frontbox_core::prelude::*;

/// Ball management is the single owner of the balls: how many there are, where they are, and getting them into play,
/// along with all of the ball handling hardware for its machine (e.g. trough, plunge lane, autoplunger, launch buttons)
/// It spawns whatever child systems it needs to run it. Other systems talk to ball management directly, not child systems.
///
/// Other systems find it by this trait, `ctx.systems.get_dyn::<dyn BallManager>()` or `ctx.ball_manager()`, so
/// an implementation lists it in [`System::provides`].
///
/// ## Responsibilities
/// - **Counting balls.** Knows how many balls are in play, in the trough, resting in holders (scoops, VUKs), and locked
///   (ball locks). A held ball is still in play: it's only resting somewhere known, and will return to the playfield.
///   A locked ball is out of play until its lock releases it. Counts
///   are derived from what the hardware reports, not from running counters, so that a halted event, failed eject, or
///   missed drain can't make them drift. Balls that appear unexpectedly are reconciled, not reported as drains.
///   Comparing against the balls installed (`configs::BALLS_INSTALLED`) tells it which are missing.
/// - **Reporting drains.** Emits `BallsExitedPlay` with how many balls are still in play. It does not decide what a drain
///   means: saving a ball is up to the participants of the game manager's `BallDraining` routine, and ending a turn
///   or a multiball to the systems watching how it completes.
/// - **Putting balls into play.** Owns the queue of balls requested. Balls are launched one at a time, spaced so the
///   hardware can receive each (e.g. waiting for the plunge lane to clear). Callers state intent with `Launch`, and the
///   implementation adapts it to the hardware, so callers never need to know what the machine has. Each request
///   returns an id, and `EjectCompleted` carries it once all of that request's balls are in play.
/// - **Stopping on request.** `cancel_ejects` drops balls not yet ejected and completes their requests. A ball resting
///   in the plunge lane stops being in play and serves the next request instead. It is called explicitly (e.g. when a
///   turn ends), as with `establish_ball_count`.
/// - **Reporting launches.** Emits `BallEnteredPlay` once a ball has actually gone into play, which can be well after it
///   was requested (e.g. a player waiting to plunge). Anything timed from a ball entering play should start here.
/// - **Establishing the ball count on request.** `establish_ball_count` takes the balls present as all there are. It
///   is called explicitly (e.g. at game start) rather than ball management listening for game events, so it does
///   not depend on game management.
///
/// ## Not responsible for
/// - Game rules: turns, ball save, multiball, extra balls. These are built on ball management's events and calls.
/// - Physically holding or releasing balls in scoops, VUKs and locks. The systems running them report what they hold
///   with `set_balls_held` or `set_balls_locked`.
///
/// ## Events
/// - `BallsExitedPlay` - Emitted when one or more balls leave play
/// - `BallEnteredPlay` - Emitted when a ball has been launched into play
/// - `EjectCompleted` - Emitted when every ball from an `eject` or `eject_many` request is in play or given up on
/// - `MaxBallsInPlayChanged` - Emitted when the number of balls that could be in play changes (e.g. a ball is locked,
///   released from a lock, found or lost)
/// - `BallsHeldChanged` - Emitted when a holder reports a change in the balls resting in it
pub trait BallManager: SpawnableSystem {
  /// Puts a ball into play. Balls are queued and launched one at a time. Returns the id `EjectCompleted` will carry.
  fn eject(&mut self, launch: Launch, ctx: &ServiceContext) -> u64;
  /// Puts `count` balls into play. Returns the id `EjectCompleted` will carry once they all are.
  fn eject_many(&mut self, count: u8, launch: Launch, ctx: &ServiceContext) -> u64;
  /// True if `Launch::Auto` can launch a ball without the player
  fn can_auto_launch(&self) -> bool;
  /// Stops putting balls into play: drops queued balls and completes every open request with the balls launched so
  /// far. A ball resting in the plunge lane is no longer in play, and serves the next request. Typically called when
  /// play stops (e.g. a turn ending or a tilt), so balls aren't launched onto a dead playfield or into the next turn.
  fn cancel_ejects(&mut self, ctx: &ServiceContext);
  /// True while a requested ball hasn't left the trough yet. Once it has, it counts toward `balls_in_play`.
  fn ejects_pending(&self) -> bool;

  /// Balls currently in play, including balls resting in holders but not locked balls
  fn balls_in_play(&self, ctx: &ServiceContext) -> u8;
  /// True while a ball is resting in the plunge lane, e.g. waiting for the player to plunge it
  fn is_ball_in_plunge_lane(&self, ctx: &ServiceContext) -> bool;
  /// Takes the balls currently present (in the trough, the plunge lane, holders and locks) as all of the balls there are.
  /// Typically called at the start of a game, so that a ball lost in a previous game isn't waited on forever. Balls
  /// installed but not found are then `balls_missing`.
  fn establish_ball_count(&mut self, ctx: &ServiceContext);
  /// Balls installed (`configs::BALLS_INSTALLED`) but not found when the count was last established, less any that
  /// turned up since
  fn balls_missing(&self, ctx: &ServiceContext) -> u8;
  /// Takes a ball in play out of the count, as lost (e.g. a ball search couldn't find it), so play doesn't wait for it.
  /// It's counted again if it turns up. Does nothing when every ball in play is held, or none is.
  fn remove_missing_ball(&mut self, ctx: &ServiceContext);

  /// Reports how many balls are resting in `holder` (its `ctx.id()`) right now: a scoop, a VUK. Read from the
  /// holder's own switches, and reported on every change, including back to 0.
  ///
  /// Held balls are still in play, as they'll be returned to the playfield: a drain with one held doesn't end the
  /// turn, and the end of a turn waits for it to drain. Knowing where they are, a ball search doesn't look for them.
  fn set_balls_held(&mut self, holder: u64, count: u8, ctx: &ServiceContext);
  /// Balls resting in holders. They're counted in `balls_in_play`.
  fn balls_held(&self) -> u8;

  /// Reports how many balls are locked in `locker` (its `ctx.id()`) right now: a ball lock that keeps them for later,
  /// such as a multiball. Read from the lock's own switches, and reported on every change, including back to 0.
  ///
  /// Unlike held balls, locked balls are out of play until released: a drain of the last ball not locked ends the
  /// turn, and the end of a turn or a game doesn't wait for them. Locking the last ball in play leaves none, so the
  /// locker should put another into play (`eject`), or the game manager will take it for a drain. Releasing one puts
  /// it back in play. Either way, `MaxBallsInPlayChanged` is emitted.
  fn set_balls_locked(&mut self, locker: u64, count: u8, ctx: &ServiceContext);
  /// Balls locked out of play. They're not counted in `balls_in_play`.
  fn balls_locked(&self) -> u8;
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
