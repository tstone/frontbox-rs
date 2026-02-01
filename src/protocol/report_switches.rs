use crate::protocol::{FastResponse, FastResponseError, SwitchState};

pub fn request() -> String {
  "SA:\r".to_string()
}

pub fn response(data: &str) -> Result<FastResponse, FastResponseError> {
  let parts: Vec<&str> = data.split(',').collect();
  let raw = parts[1];
  let mut states = Vec::new();

  // state information comes in as an ASCII string which when converted to binary gives the complete state
  // e.g. SA:OE,01F46AC100000000000000000000

  for char in raw.chars() {
    let value = char.to_digit(16).ok_or(FastResponseError::InvalidFormat)?;
    for bit in 0..4 {
      // TODO: verify MSB/LSB
      let state = if (value & (1 << (3 - bit))) != 0 {
        SwitchState::Closed
      } else {
        SwitchState::Open
      };
      states.push(state);
    }
  }

  Ok(FastResponse::SwitchReport { switches: states })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_response() {
    let result = response("OE,01F");

    let expected_switches = vec![
      SwitchState::Open,   // 0
      SwitchState::Open,   // 1
      SwitchState::Open,   // 2
      SwitchState::Open,   // 3
      SwitchState::Open,   // 4
      SwitchState::Open,   // 5
      SwitchState::Open,   // 6
      SwitchState::Closed, // 7
      SwitchState::Closed, // 8
      SwitchState::Closed, // 9
      SwitchState::Closed, // 10
      SwitchState::Closed, // 11
    ];

    assert_eq!(
      result,
      Ok(FastResponse::SwitchReport {
        switches: expected_switches
      })
    );
  }
}
