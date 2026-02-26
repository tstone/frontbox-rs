mod hardware_definition;
mod led;
mod machine;
pub mod plugins;

pub use crate::hardware_definition::*;
pub use crate::machine::districts;
pub use crate::machine::store;

pub mod prelude {
  pub use crate::districts::*;
  pub use crate::hardware_definition::*;
  pub use crate::led::*;
  pub use crate::machine::config_value::{ConfigItem, ConfigValue};
  pub use crate::machine::context::Context;
  pub use crate::machine::machine::*;
  pub use crate::machine::machine_builder::*;
  pub use crate::machine::machine_command::MachineCommand;
  pub use crate::machine::machine_config::{MachineConfig, default_config};
  pub use crate::machine::plugin::*;
  pub use crate::machine::switch_context::SwitchContext;
  pub use crate::machine::system::*;
  pub use crate::machine::system_timer::TimerMode;
  pub use crate::store::*;

  // re-exports
  pub use crossterm::event::KeyCode;
  pub use crossterm::event::MediaKeyCode;
  pub use crossterm::event::ModifierKeyCode;
  pub use fast_protocol::{Color, DriverConfig, DriverTriggerControlMode, LedType, Power};
  pub use frontbox_storable_derive::Storable;
  pub use serde::Serialize;
  pub use std::time::Duration;

  pub type RuntimeType = std::any::TypeId;
}
