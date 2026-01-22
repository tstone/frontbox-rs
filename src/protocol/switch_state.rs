use crate::protocol::{FastResponse, FastResponseError};

pub fn open_response(data: &str) -> Result<FastResponse, FastResponseError> {
  // convert hex string into decimal ID
  match usize::from_str_radix(data, 16) {
    Ok(id) => Ok(FastResponse::SwitchOpened { switch_id: id }),
    Err(_) => Err(FastResponseError::InvalidFormat),
  }
}

pub fn closed_response(data: &str) -> Result<FastResponse, FastResponseError> {
  // convert hex string into decimal ID
  match usize::from_str_radix(data, 16) {
    Ok(id) => Ok(FastResponse::SwitchClosed { switch_id: id }),
    Err(_) => Err(FastResponseError::InvalidFormat),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_open_response() {
    let result = open_response("1A");
    assert_eq!(result, Ok(FastResponse::SwitchOpened { switch_id: 26 }));
  }

  #[test]
  fn test_closed_response() {
    let result = closed_response("FF");
    assert_eq!(result, Ok(FastResponse::SwitchClosed { switch_id: 255 }));
  }
}
