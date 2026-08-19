//! # Machine
//! 
//! <div class="warning">Stability Level: High</div>
//! 
//! Machine handles all the interactions with the FAST mainboard (e.g. Neuron) and the hardware connected to it. This allows operations like activating and deactivating coils, setting LED state, etc.
//! 
//! `Machine` itself is actually a special [system](mod@crate::systems) automatically launched by the framwork. It can be interacted with as a service. There are also [Context](crate::systems::Context) extensions for most common actions.
//! 
//! ```rust
//! // short hand
//! ctx.deactivate_driver(driver, mode);
//! 
//! // long hand
//! ctx.expect::<Machine>()
//!   .deactivate_driver(driver, mode, ctx);
//! ```

mod fast_codec;
mod machine;
mod machine_ext;
pub(crate) mod serial_interface;

mod events;

pub use events::*;
pub use machine::{Machine, MachinePort};
pub(crate) use machine::MachineImpl;
pub use machine_ext::*;