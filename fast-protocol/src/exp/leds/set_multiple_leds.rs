use crate::*;

/// Set multiple LEDs the same color (RP)
#[derive(Debug, Clone)]
pub struct SetMultipleLedsCommand {
  address: ExpAddress,
  color: Rgb,
  indexes: Vec<u16>,
}

impl SetMultipleLedsCommand {
  pub fn new(address: ExpAddress, color: Rgb, indexes: Vec<u16>) -> Self {
    Self {
      address,
      color,
      indexes,
    }
  }
}

impl FastStringDispatch for SetMultipleLedsCommand {
  fn to_string(&self) -> String {
    // https://fastpinball.com/fast-serial-protocol/exp/rp/
    let indexes = self
      .indexes
      .iter()
      .map(|idx| format!("{:X}", idx))
      .collect::<Vec<_>>()
      .join(",");
    format!("RP@{}:{},{}\r", self.address, self.color.to_hex(), indexes)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let result =
      SetMultipleLedsCommand::new(ExpAddress::new(0x48, None), Rgb::new(255, 0, 0), vec![0, 1])
        .to_string();
    assert_eq!(result, "RP@48:FF0000,0,1\r");
  }
}
