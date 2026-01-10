pub mod configure_hardware;
pub mod id;
pub mod watchdog;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum FastResponse {
  Unrecognized(String),
  Failed(String),
  Invalid(String),

  IdResponse {
    processor: String,
    product_number: String,
    firmware_version: String,
  },

  WatchdogProcessed,
  WatchdogDisabled,
  WatchdogExpired,
  WatchdogRemaining(u16),
}

pub fn parse(line: String) -> Option<FastResponse> {
  let (prefix, mut suffix) = line.split_at(3);
  suffix = suffix.trim_end();

  if suffix == "F" {
    return Some(FastResponse::Failed(prefix[..2].to_string()));
  } else if suffix == "X" {
    return Some(FastResponse::Invalid(prefix[..2].to_string()));
  }

  let msg = if prefix == "ID:" {
    id::response(suffix)
  } else if prefix == "WD:" {
    watchdog::response(suffix)
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
