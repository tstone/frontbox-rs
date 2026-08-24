//! # Frontbox
//! Frontbox is a homebrew arcade framework built for [FAST Pinball](https://fastpinball.com/) hardware,
//! designed around an actor-like constructs called "Systems", which send and receive signal.
//!
//! ## Getting Started
//! 1. Define your [app](mod@crate::app).
//! 2. Define your [hardware](mod@crate::hardware).
//! 3. Write a [system](mod@crate::systems).
//! 4. Emit some [events](mod@crate::systems::event).
//! 5. Turn on some [LEDs](mod@crate::led).
//! 6. [Choreograph](mod@crate::systems::cue) something.

pub mod animation;
pub mod app;
mod cycle;
mod duration_ext;
pub mod extent;
pub mod hardware;
pub mod led;
pub mod machine;
mod macros;
pub mod operator_config;
pub mod provided;
mod store;
pub mod systems;

pub use hardware::tags;

// frontbox/src/lib.rs
extern crate self as frontbox;

pub mod prelude {
  pub use crate::app::*;
  pub use crate::cycle::*;
  pub use crate::duration_ext::*;
  pub use crate::events;
  pub use crate::extent::*;
  pub use crate::hardware::*;
  pub use crate::hardware_defs;
  pub use crate::led::color_sequence::GradientStop;
  pub use crate::led::*;
  pub use crate::machine::*;
  pub use crate::operator_config::*;
  pub use crate::store::*;
  pub use crate::systems;
  pub use crate::systems::*;

  // re-exports
  pub use fast_protocol::driver_config::*;
  pub use fast_protocol::{DriverTriggerControlMode, LedType, Power};
  pub use frontbox_derive::*;
  pub use glam::{Quat, Vec2, Vec3};
  pub use image::Rgba;
  pub use indexmap::IndexSet;
  pub use serde::Serialize;
  pub use std::time::Duration;

  pub type RuntimeType = std::any::TypeId;
}
