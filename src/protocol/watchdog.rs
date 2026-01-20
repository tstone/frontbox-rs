use std::time::Duration;

use crate::protocol::{FastResponse, FastResponseError};

pub fn get() -> String {
  "WD:\r".to_string()
}

pub fn set(duration: Duration) -> String {
  // Convert decimal to hex string for milliseconds
  format!("WD:{:X}\r", duration.as_millis() / 1000)
}

pub fn end() -> String {
  "WD:0\r".to_string()
}

pub fn response(data: &str) -> Result<FastResponse, FastResponseError> {
  if data == "00000000" {
    Ok(FastResponse::WatchdogDisabled)
  } else if data == "FFFFFFFF" {
    Ok(FastResponse::WatchdogExpired)
  } else {
    match data.parse::<u64>() {
      Ok(remaining) => Ok(FastResponse::WatchdogRemaining(Duration::from_millis(
        remaining,
      ))),
      Err(_) => Err(FastResponseError::InvalidFormat),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_set_with_time() {
    let result = set(Duration::from_millis(1500));
    assert_eq!(result, "WD:5DC\r");
  }

  #[test]
  fn test_response_disabled() {
    let result = response("00000000");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FastResponse::WatchdogDisabled);
  }

  #[test]
  fn test_response_expired() {
    let result = response("FFFFFFFF");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), FastResponse::WatchdogExpired);
  }
}
