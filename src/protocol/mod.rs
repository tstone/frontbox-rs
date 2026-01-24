pub mod configure_driver;
pub mod configure_hardware;
pub mod driver_trigger;
mod error;
pub mod id;
pub mod switch_state;
pub mod watchdog;

use std::time::Duration;

pub use error::FastResponseError;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum FastResponse {
  Unrecognized(String),
  Failed(String),
  Invalid(String),
  Processed(String),

  IdResponse {
    processor: String,
    product_number: String,
    firmware_version: String,
  },

  SwitchOpened {
    switch_id: usize,
  },
  SwitchClosed {
    switch_id: usize,
  },

  WatchdogDisabled,
  WatchdogExpired,
  WatchdogRemaining(Duration),
}

pub fn parse(line: String) -> Option<FastResponse> {
  let (prefix, mut suffix) = line.split_at(3);
  suffix = suffix.trim_end();

  if suffix == "F" {
    return Some(FastResponse::Failed(prefix[..2].to_string()));
  } else if suffix == "X" {
    return Some(FastResponse::Invalid(prefix[..2].to_string()));
  } else if suffix == "P" {
    return Some(FastResponse::Processed(prefix[..2].to_string()));
  }

  let msg = if prefix == "ID:" {
    id::response(suffix)
  } else if prefix == "WD:" {
    watchdog::response(suffix)
  } else if prefix == "-L:" {
    switch_state::closed_response(suffix)
  } else if prefix == "/L:" {
    switch_state::open_response(suffix)
  } else {
    Ok(FastResponse::Unrecognized(line.clone()))
  };

  match msg {
    Ok(response) => Some(response),
    Err(e) => {
      log::error!("Error parsing response '{}': {:?}", line, e);
      None
    }
  }
}
