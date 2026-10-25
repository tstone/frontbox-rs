use super::{MAX_BINARY_COUNT, describe_binary};
use crate::common::expansion_addr;
use crate::*;
use std::any::Any;

/// Set multiple LEDs the same color, binary encoded (RB)
///
/// LED indexes are `0x00`-`0x7F`, or `0x80`-`0xFF` for mapped LED sets created with `LM:`.
#[derive(Debug, Clone)]
pub struct SetMultipleLedsBinaryCommand {
  expansion_board: u8,
  breakout: Option<u8>,
  color: Color,
  indexes: Vec<u8>,
}

impl SetMultipleLedsBinaryCommand {
  pub fn new(expansion_board: u8, breakout: Option<u8>, color: Color, indexes: Vec<u8>) -> Self {
    Self {
      expansion_board,
      breakout,
      color,
      indexes,
    }
  }
}

impl FastBinaryDispatch for SetMultipleLedsBinaryCommand {
  fn to_bytes(&self) -> Vec<u8> {
    // https://fastpinball.com/fast-serial-protocol/exp/rb/
    // The color sits between the count and the indexes, so it can't go through `binary_frames`
    let address = expansion_addr(self.expansion_board, self.breakout);
    let mut bytes = Vec::new();
    for chunk in self.indexes.chunks(MAX_BINARY_COUNT) {
      bytes.extend(format!("RB@{}:", address).as_bytes());
      bytes.extend([chunk.len() as u8, self.color.r, self.color.g, self.color.b]);
      bytes.extend(chunk);
    }
    bytes
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn describe(&self) -> String {
    describe_binary(
      "RB",
      self.expansion_board,
      self.breakout,
      std::iter::once(self.color.to_hex())
        .chain(self.indexes.iter().map(|idx| format!("{:02X}", idx))),
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let result = SetMultipleLedsBinaryCommand::new(
      0x48,
      None,
      Color::rgb(0xFF, 0xFF, 0x00),
      vec![0x00, 0x01, 0x02, 0x03, 0x04],
    )
    .to_bytes();

    let mut expected = b"RB@48:".to_vec();
    expected.extend([0x05, 0xFF, 0xFF, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04]);
    assert_eq!(result, expected);
  }

  #[test]
  fn test_describe() {
    let result =
      SetMultipleLedsBinaryCommand::new(0x48, None, Color::rgb(0xFF, 0xFF, 0x00), vec![0x00, 0x1A])
        .describe();
    assert_eq!(result, "RB@48:FFFF00,00,1A");
  }
}
