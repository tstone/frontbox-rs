pub mod animation;
mod app;
mod hardware;
mod led;
mod machine;
mod macros;
mod operator_config;
pub mod plugins;
mod store;
pub mod systems;

pub use crate::hardware::*;

pub mod prelude {
  pub use crate::app::*;
  pub use crate::events;
  pub use crate::hardware::*;
  pub use crate::hardware_defs;
  pub use crate::led::*;
  pub use crate::machine::event_interrupt_registry::InterruptResult;
  pub use crate::machine::machine::*;
  pub use crate::machine::machine_commands::*;
  pub use crate::machine::machine_ext::*;
  pub use crate::operator_config::*;
  pub use crate::plugins::Plugin;
  pub use crate::store::*;
  pub use crate::systems;
  pub use crate::systems::*;

  // re-exports
  pub use fast_protocol::driver_config::*;
  pub use fast_protocol::{DriverTriggerControlMode, LedType, Power};
  pub use frontbox_derive::*;
  pub use glam::{Quat, Vec2, Vec3};
  pub use image::Rgba;
  pub use serde::Serialize;
  pub use std::time::Duration;

  pub type RuntimeType = std::any::TypeId;
}
