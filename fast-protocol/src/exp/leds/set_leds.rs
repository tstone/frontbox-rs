use crate::*;

/// Set the color of multiple LEDs in a single command (RS)
#[derive(Debug, Clone)]
pub struct SetLedsCommand {
  address: ExpAddress,
  states: Vec<(u16, Rgb)>,
}

impl SetLedsCommand {
  pub fn new(address: ExpAddress, states: Vec<(u16, Rgb)>) -> Self {
    Self { address, states }
  }
}

impl FastStringDispatch for SetLedsCommand {
  fn to_string(&self) -> String {
    // https://fastpinball.com/fast-serial-protocol/exp/rs/
    let states_part = self
      .states
      .iter()
      .map(|(led_idx, color)| format!("{:02X}{}", led_idx, color.to_hex()))
      .collect::<Vec<_>>()
      .join(",");
    format!("RS@{}:{}\r", self.address, states_part)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let result = SetLedsCommand::new(
      ExpAddress::new(0x48, None),
      vec![(0, Rgb::new(255, 0, 0)), (1, Rgb::new(0, 255, 0))],
    )
    .to_string();
    assert_eq!(result, "RS@48:00FF0000,0100FF00\r");
  }
}
