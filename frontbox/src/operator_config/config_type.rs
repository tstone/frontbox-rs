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

/// Stored as 0-255, shown as a percentage
impl ConfigType for Power {
  fn to_value(&self) -> ConfigDomainValue {
    ConfigDomainValue::Int(self.power as i64)
  }

  fn from_value(value: ConfigDomainValue) -> Self {
    Power {
      power: expect_int(value).clamp(0, 255) as u8,
    }
  }

  fn display(&self) -> String {
    format!("{}%", ((self.power as f32 / 255.0) * 100.0) as u8)
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
