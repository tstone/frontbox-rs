mod hardware;
mod machine;
mod mainboard;
pub mod plugins;

// --- protocol ----

#[cfg(feature = "protocol")]
pub mod protocol;

// --- machine ---

pub use crate::machine::runtimes;
pub use crate::machine::store;
pub use crate::mainboard::*;

pub mod prelude {
  pub use crate::machine::command::*;
  pub use crate::machine::context::*;
  pub use crate::machine::game_state::*;
  pub use crate::machine::machine::*;
  pub use crate::machine::machine_builder::*;
  pub use crate::machine::plugin::*;
  pub use crate::machine::switch_context::SwitchContext;
  pub use crate::machine::system::*;
  pub use crate::mainboard::BootConfig;
  pub use crate::mainboard::FastIoBoards;
  pub use crate::mainboard::FastPlatform;
  pub use crate::mainboard::IoNetworkSpec;
  pub use crate::mainboard::SwitchConfig;

  pub use crossterm::event::KeyCode;

  pub type MachineModeType = std::any::TypeId;
}
