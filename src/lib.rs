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
  pub use crate::machine::config_value::{ConfigItem, ConfigValue};
  pub use crate::machine::context::Context;
  pub use crate::machine::machine::*;
  pub use crate::machine::machine_builder::*;
  pub use crate::machine::machine_config::MachineConfig;
  pub use crate::machine::plugin::*;
  pub use crate::machine::switch_context::SwitchContext;
  pub use crate::machine::system::*;
  pub use crate::machine::system_timer::TimerMode;
  pub use crate::mainboard::BootConfig;
  pub use crate::mainboard::FastIoBoards;
  pub use crate::mainboard::FastPlatform;
  pub use crate::mainboard::IoNetworkSpec;
  pub use crate::mainboard::SwitchConfig;
  pub use crate::protocol::prelude::{DriverConfig, DriverTriggerControlMode, Power};
  pub use crate::runtimes::*;
  pub use crate::store::Store;

  pub use crossterm::event::KeyCode;
  pub use crossterm::event::MediaKeyCode;
  pub use crossterm::event::ModifierKeyCode;

  pub type RuntimeType = std::any::TypeId;
}
