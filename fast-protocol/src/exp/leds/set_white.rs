use crate::*;

/// Set the white channel of one or more LEDs on an expansion board. This command is used for RGBW LEDs, and will
/// only affect the white channel, leaving the RGB channels unchanged.
#[derive(Debug, Clone)]
pub struct SetWhiteCommand {
  address: ExpAddress,
  states: Vec<(u16, u8)>,
}

impl SetWhiteCommand {
  pub fn new(address: ExpAddress, states: Vec<(u16, u8)>) -> Self {
    Self { address, states }
  }
}

impl FastStringDispatch for SetWhiteCommand {
  fn to_string(&self) -> String {
    // https://fastpinball.com/fast-serial-protocol/exp/rw/
    let states_part = self
      .states
      .iter()
      .map(|(led_idx, color)| format!("{:X}{:X}", led_idx, color))
      .collect::<Vec<_>>()
      .join(",");
    format!("RW@{}:{}\r", self.address, states_part)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let result =
      SetWhiteCommand::new(ExpAddress::new(0x48, None), vec![(0, 0xFF), (1, 0x80)]).to_string();
    assert_eq!(result, "RW@48:0FF,180\r");
  }
}
