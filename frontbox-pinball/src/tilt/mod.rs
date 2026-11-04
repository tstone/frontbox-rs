mod slam_tilt_system;
mod tilt_ext;
mod tilt_system;

pub use slam_tilt_system::*;
pub use tilt_ext::*;
pub use tilt_system::*;

use frontbox::prelude::*;

/// Emitted when the tilt bob swings and the player is warned, rather than tilted
#[derive(serde::Serialize, Event)]
pub struct TiltWarned {
  /// Warnings left before the next swing tilts
  pub remaining: u8,
}

/// Emitted when the slam tilt switch closes, during a game or not. During a game, the game also ends with
/// `GameEndReason::SlamTilted`.
#[derive(serde::Serialize, Event)]
pub struct SlamTilted;
