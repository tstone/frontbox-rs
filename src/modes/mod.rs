pub mod freeplay;
pub mod game_state;
pub mod machine_context;
pub mod machine_mode;

pub mod prelude {
  pub use crate::modes::freeplay::Freeplay;
  pub use crate::modes::game_state::GameState;
  pub use crate::modes::machine_context::MachineContext;
  pub use crate::modes::machine_mode::MachineMode;

  // re-exports
  pub use crate::machine::Switch;
}
