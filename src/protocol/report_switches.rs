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
      payload: "OE,01F".to_string(),
      ..Default::default()
    });

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
      Ok(SwitchReportResponse::SwitchReport {
        switches: expected_switches
      })
    );
  }
}
