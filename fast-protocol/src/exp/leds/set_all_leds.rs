use crate::common::expansion_addr;
use crate::*;

/// Set all LEDs on a port/breakout to the same color (RA)
#[derive(Debug, Clone)]
pub struct SetAllLedsCommand {
  expansion_board: u8,
  breakout: Option<u8>,
  color: Color,
}

impl SetAllLedsCommand {
  pub fn new(expansion_board: u8, breakout: Option<u8>, color: Color) -> Self {
    Self {
      expansion_board,
      breakout,
      color,
    }
  }
}

impl FastStringCommand for SetAllLedsCommand {
  fn to_string(&self) -> String {
    // https://fastpinball.com/fast-serial-protocol/exp/ra/
    let address = expansion_addr(self.expansion_board, self.breakout);
    format!("RA@{}:{}\r", address, self.color.to_hex())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let result = SetAllLedsCommand::new(0x48, None, Color::rgb(255, 0, 0)).to_string();
    assert_eq!(result, "RA@48:FF0000\r");
  }
}
