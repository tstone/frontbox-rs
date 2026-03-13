mod hardware_definition;
mod led;
#[macro_use]
mod macros;
mod commands;
mod machine;
pub mod plugins;
mod states;
pub mod systems;

pub use crate::hardware_definition::*;
pub use crate::machine::store;

pub mod prelude {
  pub use crate::commands::*;
  pub use crate::handle_event;
  pub use crate::hardware_definition::*;
  pub use crate::led::*;
  pub use crate::machine::config_value::{ConfigItem, ConfigValue};
  pub use crate::machine::context::Context;
  pub use crate::machine::event::*;
  pub use crate::machine::machine::*;
  pub use crate::machine::machine_builder::*;
  pub use crate::machine::machine_command::MachineCommand;
  pub use crate::machine::machine_config::{MachineConfig, default_config};
  pub use crate::machine::plugin::*;
  pub use crate::machine::switch_context::SwitchContext;
  pub use crate::states::*;
  pub use crate::store::*;
  pub use crate::systems::{CloneableSystem, OnEventSystem, System, SystemTimer, TimerMode};

  // re-exports
  pub use crossterm::event::KeyCode;
  pub use crossterm::event::MediaKeyCode;
  pub use crossterm::event::ModifierKeyCode;
  pub use fast_protocol::driver_config::*;
  pub use fast_protocol::{Color, DriverTriggerControlMode, LedType, Power};
  pub use frontbox_derive::*;
  pub use serde::Serialize;
  pub use std::time::Duration;

  pub type RuntimeType = std::any::TypeId;
}
