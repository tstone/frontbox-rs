use crate::plugins::{Plugin, TroughSystem};
pub use crate::prelude::*;

pub struct TroughPlugin {
  pub switches: Vec<&'static str>,
  pub eject_coil: &'static str,
  pub expected_occupancy: usize,
}

impl TroughPlugin {
  pub fn trough_kick_key() -> &'static str {
    "trough_kick"
  }

  pub fn trough_power_key() -> &'static str {
    "trough_power"
  }

  /// The plugin will monitor the specified switches to track the occupancy of the trough, and fire the eject coil when the trough is full and a new ball enters.
  ///
  /// ## Outputs
  /// - Event: `BallEnteredTrough` - Emitted when a ball enters the trough
  /// - Event: `BallExitedTrough` - Emitted when a ball exits the trough
  /// - Event: `TroughFull` - Emitted when the trough reaches full occupancy
  ///
  /// ## Inputs
  /// - Command: `TroughEject` - Fires the trough eject coil to kick the ball back into play
  /// - Command: `BallRemovedFromPlay` - Signal to the trough that a ball has been removed from play (e.g. ball lock)
  /// - Command: `BallReturnedToPlay` - Signal to the trough that a ball has been returned to play (e.g. ball lock released)
  ///
  /// ## Operator Config
  /// - `trough_kick_key` (integer) - The duration to fire the trough kick at when ejecting a ball
  /// - `trough_power_key` (integer, ms) - The initial power to contact the ball
  ///
  /// ## Arguments
  /// * `switches` - List of trough switches, in order. Index 0 is the switch nearest the exit.
  /// * `eject_coil` - The name of the trough eject coil
  pub fn new(switches: Vec<&'static str>, eject_coil: &'static str) -> Self {
    Self {
      expected_occupancy: switches.len(),
      switches,
      eject_coil,
    }
  }
}

impl Plugin for TroughPlugin {
  fn register(&self, app: &mut App) {
    app.operator_config(
      OperatorConfigs::integer(Self::trough_kick_key())
        .default(100)
        .min(0)
        .max(255)
        .name("Trough Kick Power")
        .description("The duration to fire the trough kick at")
        .units("ms"),
    );

    app.operator_config(
      OperatorConfigs::integer(Self::trough_power_key())
        .default(70)
        .min(0)
        .max(100)
        .name("Trough Power")
        .description("The initial power to contact the ball")
        .units("%"),
    );

    app.system(TroughSystem::new(self.switches.clone(), self.eject_coil));
  }
}
