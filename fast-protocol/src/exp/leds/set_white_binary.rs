use super::{binary_frames, describe_binary};
use crate::*;
use std::any::Any;

/// Set the white channel of one or more RGBW LEDs, binary encoded (RZ). Only affects the white channel, leaving the
/// RGB channels unchanged.
///
/// LED indexes are `0x00`-`0x7F`, or `0x80`-`0xFF` for mapped LED sets created with `LM:`.
#[derive(Debug, Clone)]
pub struct SetWhiteBinaryCommand {
  expansion_board: u8,
  breakout: Option<u8>,
  states: Vec<(u8, u8)>,
}

impl SetWhiteBinaryCommand {
  pub fn new(expansion_board: u8, breakout: Option<u8>, states: Vec<(u8, u8)>) -> Self {
    Self {
      expansion_board,
      breakout,
      states,
    }
  }
}

impl FastBinaryDispatch for SetWhiteBinaryCommand {
  fn to_bytes(&self) -> Vec<u8> {
    // https://fastpinball.com/fast-serial-protocol/exp/rz/
    binary_frames(
      "RZ",
      self.expansion_board,
      self.breakout,
      &self.states,
      |payload, (led_idx, white)| payload.extend([*led_idx, *white]),
    )
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn describe(&self) -> String {
    describe_binary(
      "RZ",
      self.expansion_board,
      self.breakout,
      self
        .states
        .iter()
        .map(|(led_idx, white)| format!("{:02X}{:02X}", led_idx, white)),
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let result = SetWhiteBinaryCommand::new(
      0x48,
      Some(1),
      vec![(0x01, 0x00), (0x10, 0x80), (0x2B, 0xFF)],
    )
    .to_bytes();

    let mut expected = b"RZ@481:".to_vec();
    expected.extend([0x03, 0x01, 0x00, 0x10, 0x80, 0x2B, 0xFF]);
    assert_eq!(result, expected);
  }

  #[test]
  fn test_describe() {
    let result =
      SetWhiteBinaryCommand::new(0x48, Some(1), vec![(0x01, 0x00), (0x2B, 0xFF)]).describe();
    assert_eq!(result, "RZ@481:0100,2BFF");
  }
}
