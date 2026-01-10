use crate::FastResponseError;
use crate::protocol::FastResponse;

pub fn set(seconds: Option<u16>) -> String {
  match seconds {
    // Convert decimal to hex string for seconds
    Some(t) => format!("WD:{:X}\r", t),
    None => "WD:\r".to_string(),
  }
}

pub fn response(data: &str) -> Result<FastResponse, FastResponseError> {
  if data == "P" {
    Ok(FastResponse::WatchdogProcessed)
  } else if data == "00000000" {
    Ok(FastResponse::WatchdogDisabled)
  } else if data == "FFFFFFFF" {
    Ok(FastResponse::WatchdogExpired)
  } else {
    match data.parse::<u16>() {
      Ok(remaining) => Ok(FastResponse::WatchdogRemaining(remaining)),
      Err(_) => Err(FastResponseError::InvalidFormat),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_set_with_time() {
    let result = set(Some(1500));
    assert_eq!(result, "WD:5DC\r");
  }

  #[test]
  fn test_set_without_time() {
    let result = set(None);
    assert_eq!(result, "WD:\r");
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
