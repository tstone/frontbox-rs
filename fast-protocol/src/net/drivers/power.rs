use std::fmt::{Display, UpperHex};
use std::ops::{Add, Deref, Rem, Sub};

/// 8-bit power for original coil modes
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Power {
  pub power: u8,
}

impl Power {
  pub fn percent(percent: u8) -> Self {
    let clamped = percent.min(100);
    let power = (clamped as u16 * 255 / 100) as u8;
    Self { power }
  }

  pub const FULL: Power = Power { power: 255 };
  pub const OFF: Power = Power { power: 0 };
  pub const ZERO: Power = Power { power: 0 };
}

impl Deref for Power {
  type Target = u8;
  fn deref(&self) -> &Self::Target {
    &self.power
  }
}

impl Add for Power {
  type Output = Self;
  fn add(self, rhs: Self) -> Self::Output {
    Power {
      power: self.power + rhs.power,
    }
  }
}

impl Sub for Power {
  type Output = Self;
  fn sub(self, rhs: Self) -> Self::Output {
    Power {
      power: self.power - rhs.power,
    }
  }
}

impl Rem for Power {
  type Output = Self;
  fn rem(self, rhs: Self) -> Self::Output {
    Power {
      power: self.power % rhs.power,
    }
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
