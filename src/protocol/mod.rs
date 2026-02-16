mod address;
mod common;
mod configure_driver;
mod configure_hardware;
mod configure_switch;
mod driver;
mod error;
mod event_response;
mod exp;
pub mod fast_command;
mod id;
pub mod raw_response;
mod report_switches;
mod switch_state;
mod trigger_driver;
mod watchdog;

pub use error::FastResponseError;
pub use event_response::*;

pub mod prelude {
  pub use crate::protocol::FastResponseError;
  pub use crate::protocol::address::*;
  pub use crate::protocol::common::ProcessedResponse;
  pub use crate::protocol::configure_driver::*;
  pub use crate::protocol::configure_hardware::*;
  pub use crate::protocol::configure_switch::*;
  pub use crate::protocol::driver::*;
  pub use crate::protocol::exp::*;
  pub use crate::protocol::fast_command::*;
  pub use crate::protocol::id::*;
  pub use crate::protocol::raw_response::RawResponse;
  pub use crate::protocol::report_switches::*;
  pub use crate::protocol::switch_state::*;
  pub use crate::protocol::trigger_driver::*;
  pub use crate::protocol::watchdog::*;
}
