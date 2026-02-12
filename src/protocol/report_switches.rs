use crate::protocol::{SwitchState, prelude::*};

pub struct ReportSwitchesCommand;

impl ReportSwitchesCommand {
  pub fn new() -> Self {
    ReportSwitchesCommand
  }
}

impl FastCommand for ReportSwitchesCommand {
  type Response = SwitchReportResponse;

  fn prefix() -> &'static str {
    "sa"
  }

  fn to_string(&self) -> String {
    "SA:\r".to_string()
  }

  fn parse(&self, raw: RawResponse) -> Result<Self::Response, FastResponseError> {
    if raw.payload.to_lowercase() == "f" {
      return Ok(SwitchReportResponse::Failed);
    }

    let parts: Vec<&str> = raw.payload.split(',').collect();
    let raw = parts[1];
    let mut states = Vec::new();

    // state information comes in as an ASCII string which when converted to binary gives the complete state
    // e.g. SA:OE,01F46AC100000000000000000000
    // Each pair of hex characters represents one byte (8 switches)

    let chars: Vec<char> = raw.chars().collect();
    for i in (0..chars.len()).step_by(2) {
      let high_nibble_char = chars.get(i).unwrap_or(&'0');
      let low_nibble_char = chars.get(i + 1).unwrap_or(&'0');

      let high_value = high_nibble_char
        .to_digit(16)
        .ok_or(FastResponseError::InvalidFormat)?;
      let low_value = low_nibble_char
        .to_digit(16)
        .ok_or(FastResponseError::InvalidFormat)?;

      // Combine nibbles into a byte value
      let byte_value = (high_value << 4) | low_value;

      // Each byte represents 8 switches, read bits LSB to MSB
      for bit in 0..8 {
        let state = if (byte_value & (1 << bit)) != 0 {
          SwitchState::Closed
        } else {
          SwitchState::Open
        };
        states.push(state);
      }
    }

    Ok(SwitchReportResponse::SwitchReport { switches: states })
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchReportResponse {
  SwitchReport { switches: Vec<SwitchState> },
  Failed,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_response() {
    let result = ReportSwitchesCommand.parse(RawResponse {
      payload: "OE,100F".to_string(),
      ..Default::default()
    });

    // Byte 0: '10' = 0x10 = 0b00010000 -> bit 4 set
    // Byte 1: '0F' = 0x0F = 0b00001111 -> bits 0,1,2,3 set
    // Reading LSB to MSB: bit 0→switch 0, bit 1→switch 1, ..., bit 7→switch 7
    let expected_switches = vec![
      SwitchState::Open,   // 0
      SwitchState::Open,   // 1
      SwitchState::Open,   // 2
      SwitchState::Open,   // 3
      SwitchState::Closed, // 4
      SwitchState::Open,   // 5
      SwitchState::Open,   // 6
      SwitchState::Open,   // 7
      SwitchState::Closed, // 8
      SwitchState::Closed, // 9
      SwitchState::Closed, // 10
      SwitchState::Closed, // 11
      SwitchState::Open,   // 12
      SwitchState::Open,   // 13
      SwitchState::Open,   // 14
      SwitchState::Open,   // 15
    ];

    assert_eq!(
      result,
      Ok(SwitchReportResponse::SwitchReport {
        switches: expected_switches
      })
    );
  }

  #[test]
  fn test_switch_27_closed() {
    // Test case from actual hardware: switch 0x27 (39 decimal) closed
    // Data: byte 4 is '80' = 0x80 = 0b10000000, bit 7 set
    // Byte 4 covers switches 32-39, so bit 7 → switch 39
    let result = ReportSwitchesCommand.parse(RawResponse {
      payload: "10,00000000800000000000000000000000".to_string(),
      ..Default::default()
    });

    match result {
      Ok(SwitchReportResponse::SwitchReport { switches }) => {
        // Verify switch 39 is closed and others are open
        assert_eq!(
          switches[39],
          SwitchState::Closed,
          "Switch 39 should be closed"
        );
        assert_eq!(switches[35], SwitchState::Open, "Switch 35 should be open");

        // Count closed switches to ensure only one is closed
        let closed_count = switches
          .iter()
          .filter(|s| **s == SwitchState::Closed)
          .count();
        assert_eq!(closed_count, 1, "Only one switch should be closed");
      }
      _ => panic!("Expected SwitchReportResponse::SwitchReport"),
    }
  }
}
