use std::time::Duration;

use fast_protocol::net::prelude::Power;

use crate::operator_config::ConfigDomainValue;

/// A Rust type that can be used as an operator config, by mapping itself to and from a [`ConfigDomainValue`]
pub trait ConfigType: Sized {
  fn to_value(&self) -> ConfigDomainValue;
  /// Only ever given values from this type's domain, so a mismatched variant is a bug
  fn from_value(value: ConfigDomainValue) -> Self;
  /// How the value is shown to the operator
  fn display(&self) -> String;
}

fn expect_int(value: ConfigDomainValue) -> i64 {
  match value {
    ConfigDomainValue::Int(v) => v,
    other => panic!("config type mismatch: expected an int, got {other:?}"),
  }
}

macro_rules! int_config_type {
  ($($t:ty),*) => {
    $(
      impl ConfigType for $t {
        fn to_value(&self) -> ConfigDomainValue {
          ConfigDomainValue::Int(*self as i64)
        }

        fn from_value(value: ConfigDomainValue) -> Self {
          let v = expect_int(value);
          <$t>::try_from(v).unwrap_or_else(|_| panic!("config value {v} out of range for {}", stringify!($t)))
        }

        fn display(&self) -> String {
          self.to_string()
        }
      }
    )*
  };
}

int_config_type!(u8, i8, u16, i16, u32, i32);

/// Stored as milliseconds
impl ConfigType for Duration {
  fn to_value(&self) -> ConfigDomainValue {
    ConfigDomainValue::Int(self.as_millis() as i64)
  }

  fn from_value(value: ConfigDomainValue) -> Self {
    Duration::from_millis(expect_int(value).max(0) as u64)
  }

  fn display(&self) -> String {
    humantime::format_duration(*self).to_string()
  }
}

/// Power in eighths, which are the choices of [`Ranges::full_power`](crate::operator_config::Ranges::full_power)
/// and how power is saved
pub(crate) const POWER_LEVEL_NAMES: [&str; 9] =
  ["0%", "12.5%", "25%", "37.5%", "50%", "62.5%", "75%", "87.5%", "100%"];

/// The PWM pattern for each level, with the on slices spread across the 8ms window
const POWER_LEVEL_PATTERNS: [Power; 9] = [
  Power::ZERO,
  Power::EIGHTH,
  Power::QUARTER,
  Power::THREE_EIGHTS,
  Power::HALF,
  Power { power: 0b1101_1010 },
  Power::THREE_QUARTERS,
  Power::SEVEN_EIGHTS,
  Power::FULL,
];

/// Stored as a choice of eighths, since a FAST PWM pattern's duty is the number of 1ms slices that are on. A custom
/// pattern reads as the level with the same duty.
impl ConfigType for Power {
  fn to_value(&self) -> ConfigDomainValue {
    ConfigDomainValue::Choice(self.power.count_ones() as usize)
  }

  fn from_value(value: ConfigDomainValue) -> Self {
    match value {
      ConfigDomainValue::Choice(i) => POWER_LEVEL_PATTERNS[i.min(POWER_LEVEL_PATTERNS.len() - 1)],
      other => panic!("config type mismatch: expected a choice, got {other:?}"),
    }
  }

  fn display(&self) -> String {
    POWER_LEVEL_NAMES[self.power.count_ones() as usize].to_string()
  }
}

impl ConfigType for bool {
  fn to_value(&self) -> ConfigDomainValue {
    ConfigDomainValue::Bool(*self)
  }

  fn from_value(value: ConfigDomainValue) -> Self {
    match value {
      ConfigDomainValue::Bool(v) => v,
      other => panic!("config type mismatch: expected a bool, got {other:?}"),
    }
  }

  fn display(&self) -> String {
    if *self { "On" } else { "Off" }.to_string()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn each_power_level_has_that_many_slices_on() {
    for (eighths, pattern) in POWER_LEVEL_PATTERNS.iter().enumerate() {
      assert_eq!(pattern.count_ones() as usize, eighths);
      assert_eq!(Power::from_value(pattern.to_value()), *pattern);
    }
  }

  #[test]
  fn custom_patterns_read_as_the_level_with_the_same_duty() {
    let custom = Power::raw(0b0110_0110);
    assert_eq!(custom.to_value(), ConfigDomainValue::Choice(4));
    assert_eq!(custom.display(), "50%");
  }
}
