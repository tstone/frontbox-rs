pub mod animation;
mod app;
mod hardware_definition;
mod led;
mod machine;
mod macros;
mod operator_config;
mod store;
pub mod systems;

pub use crate::hardware_definition::*;
pub use systems::prebuilt;

pub mod prelude {
  pub use crate::app::*;
  pub use crate::hardware_definition::*;
  pub use crate::led::*;
  pub use crate::machine::event_interrupt_registry::InterruptResult;
  pub use crate::machine::machine::*;
  pub use crate::machine::machine_commands::*;
  pub use crate::operator_config::*;
  pub use crate::signals;
  pub use crate::store::*;
  pub use crate::systems::*;

  // re-exports
  pub use fast_protocol::driver_config::*;
  pub use fast_protocol::{Color, DriverTriggerControlMode, LedType, Power};
  pub use frontbox_derive::*;
  pub use serde::Serialize;
  pub use std::time::Duration;

  pub type RuntimeType = std::any::TypeId;
}
