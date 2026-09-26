//! # Machine
//!
//! <div class="warning">Stability Level: High</div>
//!
//! Machine handles all the interactions with the FAST mainboard (e.g. Neuron) and the hardware connected to it. This allows operations like activating and deactivating coils, setting LED state, etc.
//!
//! `Machine` itself is actually a special [system](mod@crate::systems) automatically launched by the framework. It can be interacted with as a service. There are also [Context](crate::systems::Context) extensions for most common actions.
//!
//! ```rust
//! // short hand
//! ctx.deactivate_driver(driver, mode);
//!
//! // long hand
//! ctx.expect::<Machine>()
//!   .deactivate_driver(driver, mode, ctx);
//! ```

mod events;
mod fast_codec;
mod machine;
mod machine_ext;
mod machine_system;
pub mod neuron;
pub(crate) mod serial_interface;
pub mod vm; // virtual

use fast_protocol::{FastAnyRequestCommand, FastBinaryCommand};
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
  Dispatch {
    port: MachinePort,
    command: Box<dyn FastBinaryCommand>,
  },
  Request {
    port: MachinePort,
    command: Box<dyn FastAnyRequestCommand>,
    timeout: Duration,
  },
}
