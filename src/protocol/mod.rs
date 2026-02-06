pub mod configure_driver;
pub mod configure_hardware;
pub mod driver_trigger;
mod error;
mod fast_response;
pub mod id;
pub mod report_switches;
pub mod switch_state;
pub mod watchdog;

pub use error::FastResponseError;
pub use fast_response::*;

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
