pub mod game_mode;
pub mod game_state;
pub mod included;
pub mod machine_context;
pub mod machine_mode;

pub mod prelude {
  pub use crate::modes::game_mode::GameMode;
  pub use crate::modes::game_state::GameState;
  pub use crate::modes::included::freeplay::Freeplay;
  pub use crate::modes::machine_context::MachineContext;
  pub use crate::modes::machine_mode::MachineMode;

  // re-exports
  pub use crate::machine::Switch;
  pub use crate::switch_context::SwitchContext;
}
