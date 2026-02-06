mod hardware;
mod machine;
mod mainboard;
mod modes;
mod plugins;

// --- protocol ----

#[cfg(feature = "protocol")]
pub mod protocol;

// --- machine ---

#[cfg(feature = "machine")]
pub use crate::machine::*;

#[cfg(feature = "machine")]
pub use crate::mainboard::*;

#[cfg(feature = "machine")]
pub mod prelude {
  pub use crate::machine::command::*;
  pub use crate::machine::machine::*;
  pub use crate::machine::plugin::*;
  pub use crate::mainboard::BootConfig;
  pub use crate::mainboard::FastIoBoards;
  pub use crate::mainboard::FastPlatform;
  pub use crate::mainboard::IoNetworkSpec;
  pub use crate::mainboard::SwitchConfig;
  pub use crate::modes::prelude::*;
  pub use crossterm::event::KeyCode;
}
