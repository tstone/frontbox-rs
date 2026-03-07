use std::time::Duration;

use crate::*;

pub struct WatchdogCommand {
  duration: Option<Duration>,
}

impl WatchdogCommand {
  /// Set the watchdog timer to a specific duration. The watchdog will expire if not reset within this time.
  /// If set to 0, current watchdog timer will be ended. If not set (None) current watchdog remaining will be fetched
  pub fn set(duration: Duration) -> Self {
    Self {
      duration: Some(duration),
    }
  }

  pub fn disable() -> Self {
    Self {
      duration: Some(Duration::ZERO),
    }
  }

  pub fn remaining() -> Self {
    Self { duration: None }
  }
}

impl FastCommand for WatchdogCommand {
  type Response = WatchdogResponse;

  fn prefix() -> &'static str {
    "wd"
  }

  fn to_string(&self) -> String {
    match self.duration {
      // Convert decimal to hex string for milliseconds
      Some(dur) => format!("WD:{:X}\r", dur.as_millis()),
      None => "WD:\r".to_string(),
    }
  }

  fn parse(&self, raw: RawResponse) -> Result<Self::Response, FastResponseError> {
    response(&raw.payload)
  }
}

pub fn response(data: &str) -> Result<WatchdogResponse, FastResponseError> {
  if data.to_lowercase() == "p" {
    return Ok(WatchdogResponse::Processed);
  } else if data.to_lowercase() == "f" {
    return Ok(WatchdogResponse::Failed);
  } else if data == "00000000" {
    Ok(WatchdogResponse::WatchdogDisabled)
  } else if data == "FFFFFFFF" {
    Ok(WatchdogResponse::WatchdogExpired)
  } else {
    match data.parse::<u64>() {
      Ok(remaining) => Ok(WatchdogResponse::WatchdogRemaining(Duration::from_millis(
        remaining,
      ))),
      Err(_) => Err(FastResponseError::InvalidFormat),
    }
  }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum WatchdogResponse {
  WatchdogDisabled,
  WatchdogExpired,
  WatchdogRemaining(Duration),
  Processed,
  Failed,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_set_with_time() {
    let result = WatchdogCommand::set(Duration::from_millis(1500)).to_string();
    assert_eq!(result, "WD:5DC\r");
  }

  #[test]
  fn test_response_disabled() {
    let result = response("00000000");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), WatchdogResponse::WatchdogDisabled);
  }

  #[test]
  fn test_response_expired() {
    let result = response("FFFFFFFF");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), WatchdogResponse::WatchdogExpired);
  }
}
