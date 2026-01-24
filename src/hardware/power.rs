use std::fmt::{Display, UpperHex};

/// 8-bit power for original coil modes
pub struct Power {
  pub power: u8,
}

impl Power {
  pub fn percent(percent: u8) -> Self {
    let clamped = percent.min(100);
    let power = (clamped as u16 * 255 / 100) as u8;
    Self { power }
  }

  pub fn full() -> Self {
    Self { power: 255 }
  }

  pub fn off() -> Self {
    Self { power: 0 }
  }
}

impl Display for Power {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.power)
  }
}

impl UpperHex for Power {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:X}", self.power)
  }
}

/// 16-bit power for modern coil modes
/// TODO: Verify if this really is 16 bit
pub struct HighPower {
  pub power: u16,
}

impl HighPower {
  pub fn percent(percent: u8) -> Self {
    let clamped = percent.min(100);
    let power = (clamped as u32 * 65535 / 100) as u16;
    Self { power }
  }

  pub fn full() -> Self {
    Self { power: 65535 }
  }

  pub fn off() -> Self {
    Self { power: 0 }
  }
}

impl Display for HighPower {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.power)
  }
}

impl UpperHex for HighPower {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:X}", self.power)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_half_power() {
    let power = Power::percent(50);
    assert_eq!(power.power, 127);
    assert_eq!(format!("{}", power), "127");
    assert_eq!(format!("{:X}", power), "7F");
  }
}
