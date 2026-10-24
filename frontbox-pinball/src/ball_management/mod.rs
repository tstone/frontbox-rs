mod autoplunger;
mod ball_management;
mod ball_management_ext;
mod ball_search;
mod plunge_lane;
mod standard_ball_management;
mod standard_ball_management_config;
mod trough;
mod trough_management_config;

pub use autoplunger::*;
pub use ball_management::*;
pub use ball_management_ext::*;
pub use ball_search::*;
pub use plunge_lane::*;
pub use standard_ball_management::*;
pub use standard_ball_management_config::*;
pub use trough::*;
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

/// Emitted once a ball has gone into play, which can be well after it was ejected (e.g. a player waiting to plunge)
#[derive(serde::Serialize, Event)]
pub struct BallLaunched {
  pub method: LaunchMethod,
}

// -- Plunge Lane Events --

#[derive(serde::Serialize, Event)]
pub struct BallEnteredPlungeLane {
  pub state: PlungeLaneState,
}

impl BallEnteredPlungeLane {
  pub fn new(state: PlungeLaneState) -> Self {
    Self { state }
  }
}

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
struct TroughEjecting;

#[derive(serde::Serialize, Event)]
pub struct TroughEjectFailed;

// -- Ball Search Events --

#[derive(serde::Serialize, Event)]
pub struct PerformBallSearch;
