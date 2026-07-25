use fast_protocol::net::prelude::Power;

use crate::operator_config::Domain;
use std::ops::{Add, Sub};
use std::time::Duration;

#[derive(Clone, Debug)]
/// Config value within a certain range. Once min/max are hit it stays.
pub struct Range<T> {
  min: T,
  max: T,
  step: T,
}

impl<T> Domain<T> for Range<T>
where
  T: Add<Output = T> + Sub<Output = T> + PartialOrd + PartialEq + Copy,
{
  fn inc(&self, value: &T) -> T {
    let v = *value + self.step;
    if v > self.max { self.max } else { v }
  }

  fn dec(&self, value: &T) -> T {
    let v = *value - self.step;
    if v < self.min { self.min } else { v }
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
