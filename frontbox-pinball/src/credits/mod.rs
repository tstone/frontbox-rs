//! Coins, credits, and the start button that spends them. See [`CreditsSystem`].

mod credits_system;

pub use credits_system::*;

use frontbox_core::prelude::*;

/// The credits, or the coins toward the next one, changed: a coin was inserted, credits were added, or one was spent
/// adding a player. For a display showing the credits.
#[derive(Serialize, Event, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreditsChanged {
  pub credits: u8,
  /// Coins inserted toward the next credit, short of `configs::COINS_PER_CREDIT`
  pub coins: u8,
}

/// Start was pressed, but no player was added. For feedback, such as a sound, a display flash, or a lamp.
#[derive(Serialize, Event, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartRefused {
  pub reason: StartRefusedReason,
}

/// Why a start press added no player
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartRefusedReason {
  /// Free play is off and there's no credit
  NoCredits,
  /// A game is started that can't take another player: its first turn is over, or it's full
  GameInProgress,
  /// Play is over and `GameEnding` runs (high-score entry, say)
  GameEnding,
  /// No game is started, but balls from the last one are still coming home
  BallsReturning,
}
