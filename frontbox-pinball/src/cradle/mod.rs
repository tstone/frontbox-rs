mod cradle_system;

pub use cradle_system::*;

use frontbox_core::prelude::*;

/// Whether the player is cradling the ball on a flipper, set by `CradleSystem`. Read it with
/// `ctx.states.is(Cradle::Cradled)`, or watch `StateChanged<Cradle>`. A mode with a timer it pauses while the ball is
/// cradled checks it when it starts the timer, as the player may already be cradling. `Free` without a
/// `CradleSystem`.
#[derive(State, Serialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Cradle {
  #[default]
  Free,
  Cradled,
}
