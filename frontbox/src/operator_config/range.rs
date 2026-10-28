use fast_protocol::net::prelude::Power;

use crate::operator_config::config_type::POWER_LEVEL_NAMES;
use crate::operator_config::{ConfigDomain, ConfigDomainValue, ConfigType, IntoConfigDomain};
use std::time::Duration;

#[derive(Clone, Debug)]
/// Config value within a certain range. Once min/max are hit it stays.
pub struct Range<T> {
  min: T,
  max: T,
  step: T,
}

impl<T: ConfigType> IntoConfigDomain<T> for Range<T> {
  fn into_domain(self) -> ConfigDomain {
    let int = |v: &T| match v.to_value() {
      ConfigDomainValue::Int(v) => v,
      other => panic!("Range bounds must be ints, got {other:?}"),
    };
    ConfigDomain::Int {
      min: int(&self.min),
      max: int(&self.max),
      step: int(&self.step),
    }
  }
}

pub struct Ranges;

impl Ranges {
  pub fn duration(from_millis: u64, to_millis: u64) -> Range<Duration> {
    Range {
      min: Duration::from_millis(from_millis),
      max: Duration::from_millis(to_millis),
      step: Duration::from_millis(1),
    }
  }

  /// A duration range that steps by whole seconds
  pub fn seconds(from_secs: u64, to_secs: u64) -> Range<Duration> {
    Range {
      min: Duration::from_secs(from_secs),
      max: Duration::from_secs(to_secs),
      step: Duration::from_secs(1),
    }
  }

  /// Power from 0% to 100% in eighths, the levels FAST PWM can spread evenly over its 8ms window
  pub fn full_power() -> PowerLevels {
    PowerLevels
  }

  pub fn u8(from: u8, to: u8) -> Range<u8> {
    Range {
      min: from,
      max: to,
      step: 1,
    }
  }

  pub fn u16(from: u16, to: u16) -> Range<u16> {
    Range {
      min: from,
      max: to,
      step: 1,
    }
  }
}

/// The domain of a [`Power`] config, a choice of eighths. See [`Ranges::full_power`].
pub struct PowerLevels;

impl IntoConfigDomain<Power> for PowerLevels {
  fn into_domain(self) -> ConfigDomain {
    ConfigDomain::Choice(&POWER_LEVEL_NAMES)
  }
}
