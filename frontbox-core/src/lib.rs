//! # Frontbox
//! Frontbox is a homebrew arcade framework built for [FAST Pinball](https://fastpinball.com/) hardware,
//! designed around an actor-like constructs called "Systems", which send and receive signal.
//!
//! ## Getting Started
//! 1. Define your [app](mod@crate::app).
//! 2. Define your [hardware](mod@crate::hardware).
//! 3. Write a [system](mod@crate::systems).
//! 4. Emit some [events](mod@crate::systems::event).
//! 5. Turn on some LEDs (see `frontbox-led`).
//! 6. [Choreograph](mod@crate::systems::cue) something.

pub mod app;
mod duration_ext;
pub mod hardware;
pub mod location;
pub mod machine;
pub use frontbox_metrics as metrics;
mod macros;
pub mod operator_config;
pub mod provided;
pub mod systems;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod toml;

pub use hardware::tags;

// frontbox-core/src/lib.rs
extern crate self as frontbox_core;

pub mod prelude {
  pub use crate::app::*;
  pub use crate::app_plane;
  pub use crate::duration_ext::*;
  pub use crate::events;
  pub use crate::hardware::*;
  pub use crate::hardware_defs;
  pub use crate::location::*;
  pub use crate::machine::{ContextDrivers, ContextLeds, ContextSwitches, MachineSystem, SwitchClosed, SwitchOpened};
  pub use crate::metrics::{
    Counter, GameId, MetricDefinition, MetricKind, MetricsCollection, Sample, SampleType,
  };
  pub use crate::operator_config::*;
  pub use crate::routine;
  pub use crate::systems;
  pub use crate::systems::*;

  // re-exports
  pub use fast_protocol::driver_config::*;
  pub use fast_protocol::{DriverTriggerControlMode, LedType, Power, Rgb, SwitchState};
  pub use frontbox_derive::*;
  pub use glam::{Quat, Vec2, Vec3};
  pub use indexmap::IndexSet;
  pub use serde::Serialize;
  pub use std::time::Duration;

  pub type RuntimeType = std::any::TypeId;
}
