mod ball_management;
mod ball_management_ext;
mod ball_search;
mod trough;
mod trough_ball_management;
mod trough_management_config;

pub use ball_management::*;
pub use ball_management_ext::*;
pub use ball_search::*;
pub use trough::*;
pub use trough_ball_management::*;
pub use trough_management_config::*;

use frontbox::prelude::*;

// -- Ball Management Events --

#[derive(serde::Serialize, Event)]
pub struct MaxBallsInPlayChanged {
  pub max_in_play: u8,
  pub delta: i8,
}

#[derive(serde::Serialize, Event)]
pub struct BallsDrained {
  pub still_in_play: u8,
  pub count: u8,
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
struct TroughEjecting;

#[derive(serde::Serialize, Event)]
pub struct TroughEjectFailed;

// -- Ball Search Events --

#[derive(serde::Serialize, Event)]
pub struct PerformBallSearch;
