pub mod configure_driver;
pub mod configure_hardware;
pub mod configure_switch;
mod driver_trigger;
mod error;
pub mod fast_command;
mod event_response;
pub mod id;
pub mod raw_response;
pub mod report_switches;
pub mod switch_state;
pub mod watchdog;

pub use error::FastResponseError;
pub use event_response::*;

use crate::protocol::prelude::RawResponse;

pub mod prelude {
  pub use crate::protocol::FastResponseError;
  pub use crate::protocol::configure_driver::*;
  pub use crate::protocol::configure_hardware::*;
  pub use crate::protocol::configure_switch::*;
  pub use crate::protocol::driver_trigger::*;
  pub use crate::protocol::fast_command::FastCommand;
  pub use crate::protocol::id::*;
  pub use crate::protocol::raw_response::RawResponse;
  pub use crate::protocol::report_switches::*;
  pub use crate::protocol::switch_state::*;
  pub use crate::protocol::watchdog::*;
}

pub fn parse_event(raw: RawResponse) -> Result<EventResponse, FastResponseError> {
  if raw.prefix == "-L:" {
    switch_state::closed_response(&raw.payload)
  } else if raw.prefix == "/L:" {
    switch_state::open_response(&raw.payload)
  } else {
    log::warn!("Unknown event type '{}'", raw.prefix);
    Err(FastResponseError::UnknownPrefix(raw.prefix))
  }
}
