use fast_protocol::net::prelude::Power;

use crate::operator_config::{ConfigDomain, ConfigType, IntoConfigDomain, RawConfigValue};
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
    let int = |v: &T| match v.to_raw() {
      RawConfigValue::Int(v) => v,
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

  pub fn power(from: u8, to: u8) -> Range<Power> {
    Range {
      min: Power { power: from },
      max: Power { power: to },
      step: Power { power: 2 },
    }
  }

  /// A range allowing power from 0% to 100%
  pub fn full_power() -> Range<Power> {
    Self::power(0, 255)
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
