mod ball_management_config;
mod ball_management_system;
mod ball_search;
mod trough;

pub use ball_management_config::*;
pub use ball_management_system::*;
pub use ball_search::*;
pub use trough::*;

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

/// Emitted once the trough has settled. Balls that entered and exited within the same settling period are both
/// counted, rather than cancelling out.
#[derive(serde::Serialize, Event)]
pub struct TroughOccupancyChanged {
  pub current_occupancy: u8,
  /// Balls which entered the trough (e.g. drained)
  pub entered: u8,
  /// Balls which exited the trough (verified ejects)
  pub exited: u8,
}

impl TroughOccupancyChanged {
  /// Net change in occupancy
  pub fn delta(&self) -> i8 {
    self.entered as i8 - self.exited as i8
  }
}

#[derive(serde::Serialize, Event)]
pub struct TroughEjecting;

#[derive(serde::Serialize, Event)]
pub struct TroughEjectFailed;

// -- Ball Search Events --

#[derive(serde::Serialize, Event)]
pub struct PerformBallSearch;
