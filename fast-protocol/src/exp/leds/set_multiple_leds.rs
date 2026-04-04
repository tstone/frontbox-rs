use crate::common::expansion_addr;
use crate::*;

/// Set multiple LEDs the same color (RP)
#[derive(Debug, Clone)]
pub struct SetMultipleLedsCommand {
  expansion_board: u8,
  breakout: Option<u8>,
  color: Color,
  indexes: Vec<u16>,
}

impl SetMultipleLedsCommand {
  pub fn new(expansion_board: u8, breakout: Option<u8>, color: Color, indexes: Vec<u16>) -> Self {
    Self {
      expansion_board,
      breakout,
      color,
      indexes,
    }
  }
}

impl FastStringCommand for SetMultipleLedsCommand {
  fn to_string(&self) -> String {
    // https://fastpinball.com/fast-serial-protocol/exp/rp/
    let address = expansion_addr(self.expansion_board, self.breakout);
    let indexes = self
      .indexes
      .iter()
      .map(|idx| format!("{:X}", idx))
      .collect::<Vec<_>>()
      .join(",");
    format!("RP@{}:{},{}\r", address, self.color.to_hex(), indexes)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let result = SetMultipleLedsCommand::new(0x48, None, Color::red(), vec![0, 1]).to_string();
    assert_eq!(result, "RP@48:FF0000,0,1\r");
  }
}
