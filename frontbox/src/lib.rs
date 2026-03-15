mod app;
mod hardware_definition;
mod led;
mod machine;
pub mod plugins;
pub mod systems;

pub use crate::hardware_definition::*;
pub use crate::machine::store;

pub mod prelude {
  pub use crate::app::*;
  pub use crate::hardware_definition::*;
  pub use crate::led::*;
  pub use crate::machine::config_value::{ConfigItem, ConfigValue};
  pub use crate::machine::event::*;
  pub use crate::machine::machine::*;
  pub use crate::machine::machine_commands::*;
  pub use crate::machine::operator_config::{OperatorConfig, default_config};
  pub use crate::machine::plugin::*;
  pub use crate::store::*;
  pub use crate::systems::{ChildSystem, Context, OnEventSystem, System, SystemTimer, TimerMode};

  // re-exports
  pub use fast_protocol::driver_config::*;
  pub use fast_protocol::{Color, DriverTriggerControlMode, LedType, Power};
  pub use frontbox_derive::*;
  pub use serde::Serialize;
  pub use std::time::Duration;

  pub type RuntimeType = std::any::TypeId;
}
