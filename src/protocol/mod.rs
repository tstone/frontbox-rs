pub mod configure_driver;
pub mod configure_hardware;
pub mod configure_switch;
mod driver;
mod error;
mod event_response;
pub mod fast_command;
pub mod id;
pub mod raw_response;
pub mod report_switches;
pub mod switch_state;
mod trigger_driver;
pub mod watchdog;

pub use error::FastResponseError;
pub use event_response::*;

pub mod prelude {
  pub use crate::protocol::FastResponseError;
  pub use crate::protocol::configure_driver::*;
  pub use crate::protocol::configure_hardware::*;
  pub use crate::protocol::configure_switch::*;
  pub use crate::protocol::driver::*;
  pub use crate::protocol::fast_command::*;
  pub use crate::protocol::id::*;
  pub use crate::protocol::raw_response::RawResponse;
  pub use crate::protocol::report_switches::*;
  pub use crate::protocol::switch_state::*;
  pub use crate::protocol::trigger_driver::*;
  pub use crate::protocol::watchdog::*;
}
