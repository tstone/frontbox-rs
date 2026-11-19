use crate::*;

/// Set all LEDs on a port/breakout to the same color (RA)
#[derive(Debug, Clone)]
pub struct SetAllLedsCommand {
  address: ExpAddress,
  color: Rgb,
}

impl SetAllLedsCommand {
  pub fn new(address: ExpAddress, color: Rgb) -> Self {
    Self { address, color }
  }
}

impl FastStringDispatch for SetAllLedsCommand {
  fn to_string(&self) -> String {
    // https://fastpinball.com/fast-serial-protocol/exp/ra/
    format!("RA@{}:{}\r", self.address, self.color.to_hex())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let result = SetAllLedsCommand::new(ExpAddress::new(0x48, None), Rgb::new(255, 0, 0)).to_string();
    assert_eq!(result, "RA@48:FF0000\r");
  }
}
