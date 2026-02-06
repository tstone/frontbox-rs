pub mod context;
pub mod game_state;
pub mod included;
pub mod system;

pub mod prelude {
  pub use crate::modes::context::Context;
  pub use crate::modes::game_state::GameState;
  pub use crate::modes::included::freeplay::Freeplay;
  pub use crate::modes::system::System;

  // re-exports
  pub use crate::machine::machine::Switch;
  pub use crate::switch_context::SwitchContext;
}
