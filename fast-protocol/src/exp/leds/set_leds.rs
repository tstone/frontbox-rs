use crate::common::expansion_addr;
use crate::*;

/// Set the color of multiple LEDs in a single command (RS)
#[derive(Debug, Clone)]
pub struct SetLedsCommand {
  expansion_board: u8,
  breakout: Option<u8>,
  states: Vec<(u16, Color)>,
}

impl SetLedsCommand {
  pub fn new(expansion_board: u8, breakout: Option<u8>, states: Vec<(u16, Color)>) -> Self {
    Self {
      expansion_board,
      breakout,
      states,
    }
  }
}

impl FastStringCommand for SetLedsCommand {
  fn to_string(&self) -> String {
    // https://fastpinball.com/fast-serial-protocol/exp/rs/
    let address = expansion_addr(self.expansion_board, self.breakout);
    let states_part = self
      .states
      .iter()
      .map(|(led_idx, color)| format!("{:X}{}", led_idx, color.to_hex()))
      .collect::<Vec<_>>()
      .join(",");
    format!("RS@{}:{}\r", address, states_part)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let result =
      SetLedsCommand::new(0x48, None, vec![(0, Color::red()), (1, Color::green())]).to_string();
    assert_eq!(result, "RS@48:0FF0000,100FF00\r");
  }
}
