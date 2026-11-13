mod coin_door_system;

pub use coin_door_system::*;

use frontbox_core::prelude::*;

/// Whether the coin door is open, set by `CoinDoorSystem`. Read it with `ctx.states.is(CoinDoor::Open)`, or watch
/// `StateChanged<CoinDoor>`. Closed without a `CoinDoorSystem`.
#[derive(State, Serialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CoinDoor {
  #[default]
  Closed,
  Open,
}
