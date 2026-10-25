use super::{binary_frames, describe_binary};
use crate::*;
use std::any::Any;

/// Set the color of multiple LEDs in a single command, binary encoded (RD)
///
/// LED indexes are `0x00`-`0x7F`, or `0x80`-`0xFF` for mapped LED sets created with `LM:`.
#[derive(Debug, Clone)]
pub struct SetLedsBinaryCommand {
  expansion_board: u8,
  breakout: Option<u8>,
  states: Vec<(u8, Color)>,
}

impl SetLedsBinaryCommand {
  pub fn new(expansion_board: u8, breakout: Option<u8>, states: Vec<(u8, Color)>) -> Self {
    Self {
      expansion_board,
      breakout,
      states,
    }
  }
}

impl FastBinaryDispatch for SetLedsBinaryCommand {
  fn to_bytes(&self) -> Vec<u8> {
    // https://fastpinball.com/fast-serial-protocol/exp/rd/
    binary_frames(
      "RD",
      self.expansion_board,
      self.breakout,
      &self.states,
      |payload, (led_idx, color)| payload.extend([*led_idx, color.r, color.g, color.b]),
    )
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn describe(&self) -> String {
    describe_binary(
      "RD",
      self.expansion_board,
      self.breakout,
      self
        .states
        .iter()
        .map(|(led_idx, color)| format!("{:02X}{}", led_idx, color.to_hex())),
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let result = SetLedsBinaryCommand::new(
      0x48,
      None,
      vec![
        (0x01, Color::rgb(0xFF, 0x00, 0xFF)),
        (0x24, Color::rgb(0xFF, 0xFF, 0x00)),
        (0x7A, Color::rgb(0x00, 0x00, 0xFF)),
      ],
    )
    .to_bytes();

    let mut expected = b"RD@48:".to_vec();
    expected.extend([
      0x03, 0x01, 0xFF, 0x00, 0xFF, 0x24, 0xFF, 0xFF, 0x00, 0x7A, 0x00, 0x00, 0xFF,
    ]);
    assert_eq!(result, expected);
  }

  #[test]
  fn test_describe() {
    let result = SetLedsBinaryCommand::new(
      0x48,
      None,
      vec![
        (0x01, Color::rgb(0xFF, 0x00, 0xFF)),
        (0x24, Color::rgb(0xFF, 0xFF, 0x00)),
      ],
    )
    .describe();
    assert_eq!(result, "RD@48:01FF00FF,24FFFF00");
  }

  #[test]
  fn test_request_splits_over_max_count() {
    let states = (0..130).map(|i| (i as u8, Color::black())).collect();
    let result = SetLedsBinaryCommand::new(0x48, None, states).to_bytes();

    // Two frames: 127 LEDs then 3 LEDs, each LED being 4 bytes
    assert_eq!(result.len(), (6 + 1 + 127 * 4) + (6 + 1 + 3 * 4));
    assert_eq!(&result[..7], b"RD@48:\x7F");
    let second = 6 + 1 + 127 * 4;
    assert_eq!(&result[second..second + 8], b"RD@48:\x03\x7F");
  }
}
