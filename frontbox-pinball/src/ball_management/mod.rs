mod autoplunger;
mod ball_manager;
mod ball_manager_ext;
mod ball_search;
mod plunge_lane;
mod standard_ball_manager;
mod standard_ball_manager_config;
mod trough;
mod trough_management_config;

pub use autoplunger::*;
pub use ball_manager::*;
pub use ball_manager_ext::*;
pub use ball_search::*;
pub use plunge_lane::*;
pub use standard_ball_manager::*;
pub use standard_ball_manager_config::*;
pub use trough::*;
pub use trough_management_config::*;

use frontbox_core::prelude::*;

// -- Ball Management Events --

#[derive(serde::Serialize, Event)]
pub struct MaxBallsInPlayChanged {
  pub max_in_play: u8,
  pub delta: i8,
}

/// Emitted when one or more balls leave play (e.g. drain), including drains that are then saved. During a turn, the game
/// manager runs a `BallDraining` routine for each to decide whether it stands.
#[derive(serde::Serialize, Event)]
pub struct BallsExitedPlay {
  /// Balls that left play
  pub count: u8,
  /// Balls still in play afterwards. Balls are in play once they have left the trough, which includes balls resting
  /// in holders (`BallManager::set_balls_held`) and a ball in the plunge lane waiting to be launched for a request,
  /// but not one kept there for the next request (see `BallManager::cancel_ejects`).
  pub in_play: u8,
}

/// Emitted when the balls resting in holders (scoops, VUKs) change, as reported with `BallManager::set_balls_held`.
/// Held balls are still in play, so this doesn't change `in_play`.
#[derive(serde::Serialize, Event)]
pub struct BallsHeldChanged {
  /// Balls resting in all holders
  pub held: u8,
  /// Negative when a holder let a ball go
  pub delta: i8,
}

/// Emitted once a ball has gone into play, which can be well after it was ejected (e.g. a player waiting to plunge).
///
/// A ball that rolls back into the plunge lane after it was considered gone is launched again, and emits this a second
/// time. Listeners should treat it as "a ball is in play", not count it.
#[derive(serde::Serialize, Event)]
pub struct BallEnteredPlay {
  pub method: LaunchMethod,
  /// Balls in play, including this one
  pub in_play: u8,
}

/// Emitted once every ball from an `eject` or `eject_many` request has gone into play or been dropped by
/// `BallManager::cancel_ejects`. `id` is the one the request returned, so a caller can tell its own request from
/// others.
///
/// A request for more balls than the trough holds waits for balls to drain, and a ball the trough can't get out is
/// tried again (`EjectFailed`), so this can be well after the request.
#[derive(serde::Serialize, Event)]
pub struct EjectCompleted {
  pub id: u64,
  pub requested: u8,
  /// Balls that went into play. Less than `requested` if any were dropped.
  pub launched: u8,
}

/// Emitted when the trough couldn't get a requested ball out, after all of its own retries
/// (`configs::TROUGH_EJECT_RETRIES`): a jam, a weak coil, a power blip. The ball stays requested, and ball management
/// tries again after `retry_in`, waiting longer after each failure in a row, until a ball gets out or
/// `BallManager::cancel_ejects` drops it. Nothing needs doing to keep it going; this is for telling the player or
/// operator. Each one is counted in `metrics::TROUGH_EJECT_FAILURES`.
#[derive(serde::Serialize, Event)]
pub struct EjectFailed {
  /// The request the ball is for, as `EjectCompleted` will carry
  pub id: u64,
  /// Failures in a row, including this one
  pub failures: u8,
  /// How long until the next try
  pub retry_in: Duration,
}

// -- Plunge Lane Events --

/// A ball came to rest on the plunge lane switch. Ball management decides which request, if any, it serves.
#[derive(serde::Serialize, Event)]
pub struct BallEnteredPlungeLane;

/// Emitted once the ball has been gone from the plunge lane for the re-enter timeout
#[derive(serde::Serialize, Event)]
pub struct BallExitedPlungeLane;

/// A ball that left the plunge lane rolled back in before the re-enter timeout
#[derive(serde::Serialize, Event)]
struct BallReenteredPlungeLane;

// -- Autoplunger Events --

#[derive(serde::Serialize, Event)]
struct AutoPlungerFired {
  method: LaunchMethod,
}

// -- Trough Events --
// Occupancy and ejecting are internal to ball management; consumers use the ball management events above

/// Emitted once the trough has settled. Balls that entered and exited within the same settling period are both
/// counted, rather than cancelling out.
#[derive(serde::Serialize, Event)]
struct TroughOccupancyChanged {
  pub current_occupancy: u8,
  /// Balls which entered the trough (e.g. drained)
  pub entered: u8,
  /// Balls which exited the trough (verified ejects)
  pub exited: u8,
}

#[derive(serde::Serialize, Event)]
struct TroughEjectStarted;

/// The trough gave up on an eject after its retries. Ball management tries again later, and tells the game with
/// `EjectFailed`.
#[derive(serde::Serialize, Event)]
struct TroughEjectFailed;

// -- Ball Search Events --

/// Routine run for each round of a ball search (`BallSearchSystem`). Its participants are whatever can knock a stuck
/// ball loose: a scoop that kicks (unless it's holding its ball on purpose), a drop bank that resets, a diverter that
/// toggles. Each fires and completes its step right away. The routine waits between steps, so coils fire one at a
/// time and the power supply's capacitors recharge in between. The app lists them in firing order. Coils with nothing
/// to decide, like pop bumpers and slingshots, are fired by `BallSearchSystem` itself (`BallSearchSystem::coils`).
///
/// ```rust,ignore
/// app.routine::<BallSearching>(routine![LowerScoop, DropBank]);
/// ```
///
/// It's started at the root, so participants are systems there (the ones running the hardware), not in a player's
/// group.
#[derive(serde::Serialize, Event, Clone)]
pub struct BallSearching {
  /// Starting at 1
  pub round: u8,
}

/// Emitted when a ball search gives up after its last round without finding a ball. One ball is taken out of the count
/// as lost (`remove_missing_ball`), so play doesn't wait for it; the game manager serves another if it was the last.
/// A ball still in play after that is searched for in turn.
#[derive(serde::Serialize, Event)]
pub struct BallSearchFailed {
  pub rounds: u8,
  /// Balls left in play, without the lost one
  pub in_play: u8,
}
