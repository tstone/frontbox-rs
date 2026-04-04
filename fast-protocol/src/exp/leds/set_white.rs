use crate::common::expansion_addr;
use crate::*;

/// Set the white channel of one or more LEDs on an expansion board. This command is used for RGBW LEDs, and will
/// only affect the white channel, leaving the RGB channels unchanged.
#[derive(Debug, Clone)]
pub struct SetWhiteCommand {
  expansion_board: u8,
  breakout: Option<u8>,
  states: Vec<(u16, u8)>,
}

impl SetWhiteCommand {
  pub fn new(expansion_board: u8, breakout: Option<u8>, states: Vec<(u16, u8)>) -> Self {
    Self {
      expansion_board,
      breakout,
      states,
    }
  }
}

impl FastStringCommand for SetWhiteCommand {
  fn to_string(&self) -> String {
    // https://fastpinball.com/fast-serial-protocol/exp/rw/
    let address = expansion_addr(self.expansion_board, self.breakout);
    let states_part = self
      .states
      .iter()
      .map(|(led_idx, color)| format!("{:X}{:X}", led_idx, color))
      .collect::<Vec<_>>()
      .join(",");
    format!("RW@{}:{}\r", address, states_part)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let result = SetWhiteCommand::new(0x48, None, vec![(0, 0xFF), (1, 0x80)]).to_string();
    assert_eq!(result, "RW@48:0FF,180\r");
  }
}
