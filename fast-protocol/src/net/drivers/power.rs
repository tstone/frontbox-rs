use std::fmt::{Display, UpperHex};
use std::ops::{Add, Deref, Rem, Sub};

/// FAST Pinball's notion of pulse width modulated (PWM) power is describing an 8ms chunk of time using one bit representing 1ms slices.
/// For example 50% duty could be represented as every other 1ms slice "ON" `0b1010_1010` or as the first 4ms off followed by 4ms on `0b0000_1111`.
/// 
/// ## Convinience Methods
/// For convinience, power in 1/8th intervals (`EIGHTH`, `QUARTER`, `THREE_EIGHTS`) offer what are likely the most-used configuration.
/// 
/// ## Custom PWM
/// To set customer power application, use `raw` with a binary value.
/// ```rust
/// Power::raw(0b0110_0110)
/// ```
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

  /// 12.5% power (symmetrical PWM)
  pub const EIGHTH: Power = Power { power: 0b1000_0000 };
  /// 25% power (symmetrical PWM)
  pub const QUARTER: Power = Power { power: 0b0100_0100 };
  /// 37.5% power (asymmetrical PWM)
  pub const THREE_EIGHTS: Power = Power { power: 0b1001_0010 };
  /// 50% power (symmetrical PWM)
  pub const HALF: Power = Power { power: 0b1010_1010 };
  /// 75% power (symmetrical PWM)
  pub const THREE_QUARTERS: Power = Power { power: 0b1110_1110 };
  /// 87.5% power (asymmetrical PWM)
  pub const SEVEN_EIGHTS: Power = Power { power: 0b1111_1110 };
  /// 100% power
  pub const FULL: Power = Power { power: 0b1111_1111 };
  
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
