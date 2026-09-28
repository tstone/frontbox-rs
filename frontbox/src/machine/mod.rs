//! # Machine
//!
//! <div class="warning">Stability Level: High</div>
//!
//! Machine handles all the interactions with the FAST mainboard (e.g. Neuron) and the hardware connected to it. This allows operations like activating and deactivating coils, setting LED state, etc.
//!
//! [`MachineSystem`] itself is actually a special [system](mod@crate::systems) automatically launched by the framework. It can be interacted with as a service. There are also [SystemContext](crate::systems::SystemContext) extensions ([`MachineExt`]) for most common actions.
//!
//! ```rust
//! # use frontbox::prelude::*;
//! # fn example(ctx: &SystemContext) {
//! # let driver = "example_coil";
//! # let mode = DeactivationMode::Disabled;
//! // short hand
//! ctx.deactivate_driver(driver, mode);
//!
//! # let mode = DeactivationMode::Disabled;
//! // long hand
//! ctx.expect::<MachineSystem>()
//!   .deactivate_driver(driver, mode, ctx.into());
//! # }
//! ```

mod events;
mod fast_codec;
mod machine;
mod machine_ext;
mod machine_system;
pub mod neuron;
pub(crate) mod serial_interface;
pub mod vm; // virtual

use fast_protocol::{FastBinaryDispatch, FastCommand};
use std::time::Duration;

pub use events::*;
pub use machine::*;
pub use machine_ext::*;
pub use machine_system::*;

#[derive(Debug)]
pub enum MachinePort {
  Io,
  Exp,
}

pub enum MachineMessage {
  WatchdogPing,
  WatchdogClear,
  RefreshSwitchState,
  Dispatch {
    port: MachinePort,
    command: Box<dyn FastBinaryDispatch>,
  },
  Command {
    port: MachinePort,
    command: Box<dyn FastCommand>,
    timeout: Duration,
  },
}
