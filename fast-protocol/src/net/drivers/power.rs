use std::fmt::{Display, UpperHex};
use std::ops::{Add, Deref, Rem, Sub};

/// 8-bit power for original coil modes
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Power {
  pub power: u8,
}

impl Power {
  /// Set as PWM in units of 8ms with binary, e.g. 0b10101010 (on, off, on, off...) is 50% power
  pub fn raw(value: u8) -> Self {
    Self { power: value }
  }

  /// WARNING: Using an arbitrary value likely does not guarantee symmetry and may result in buzzing for longer holds
  pub fn percent(percent: u8) -> Self {
    let clamped = percent.min(100);
    let power = (clamped as u16 * 255 / 100) as u8;
    Self { power }
  }

  /// 12.5% power (symmetrical PWM)
  pub const EIGHTH: Power = Power { power: 0b10000000 };
  /// 25% power (symmetrical PWM)
  pub const QUARTER: Power = Power { power: 0b1000100 };
  /// 37.5% power (asymmetrical PWM)
  pub const THREE_EIGHTS: Power = Power { power: 0b10010010 };
  /// 50% power (symmetrical PWM)
  pub const HALF: Power = Power { power: 0b10101010 };
  /// 75% power (symmetrical PWM)
  pub const THREE_QUARTERS: Power = Power { power: 0b11101110 };
  /// 87.5% power (asymmetrical PWM)
  pub const SEVEN_EIGHTS: Power = Power { power: 0b11111110 };

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
    let power = Power::HALF;
    assert_eq!(power.power, 127);
    assert_eq!(format!("{}", power), "127");
    assert_eq!(format!("{:X}", power), "7F");
  }
}
