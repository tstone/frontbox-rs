use palette::Srgb;

use crate::protocol::common::{expansion_addr, srgb_to_hex};
use crate::protocol::prelude::*;

pub struct SetLedCommand {
  expansion_board: u8,
  breakout: Option<u8>,
  // None: Off, Some(color): On with the given color
  states: Vec<(u16, Srgb)>,
}

impl SetLedCommand {
  pub fn new(expansion_board: u8, breakout: Option<u8>, states: Vec<(u16, Srgb)>) -> Self {
    Self {
      expansion_board,
      breakout,
      states,
    }
  }
}

impl FastCommand for SetLedCommand {
  type Response = ProcessedResponse;

  fn prefix() -> &'static str {
    "rs"
  }

  fn to_string(&self) -> String {
    // https://fastpinball.com/fast-serial-protocol/exp/rs/
    let address = expansion_addr(self.expansion_board, self.breakout);
    let states_part = self
      .states
      .iter()
      .map(|(led_idx, color)| format!("{:X}{}", led_idx, srgb_to_hex(color)))
      .collect::<Vec<_>>()
      .join(",");
    format!("RS@{}:{}\r", address, states_part)
  }

  fn parse(&self, raw: RawResponse) -> Result<Self::Response, FastResponseError> {
    ProcessedResponse::parse(raw)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let result = SetLedCommand::new(
      0x48,
      None,
      vec![(0, Srgb::new(1.0, 0.0, 0.0)), (1, Srgb::new(0.0, 1.0, 0.0))],
    )
    .to_string();
    assert_eq!(result, "RS@48:0FF0000,100FF00\r");
  }
}
