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
