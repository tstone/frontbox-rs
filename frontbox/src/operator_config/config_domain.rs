/// A config value in the form the menu, persistence, and console work with, independent of its Rust type
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum ConfigDomainValue {
  Int(i64),
  Bool(bool),
  /// Index into the domain's choices
  Choice(usize),
}

/// The values a config is allowed to take. Stepping clamps at the ends rather than wrapping.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum ConfigDomain {
  Int {
    min: i64,
    max: i64,
    step: i64,
  },
  Bool,
  /// Choice names, which are also how a choice is saved
  Choice(#[cfg_attr(feature = "ts", ts(type = "Array<string>"))] &'static [&'static str]),
}

impl ConfigDomain {
  pub fn inc(&self, value: ConfigDomainValue) -> ConfigDomainValue {
    match (self, value) {
      (Self::Int { max, step, .. }, ConfigDomainValue::Int(v)) => {
        ConfigDomainValue::Int((v + step).min(*max))
      }
      (Self::Bool, ConfigDomainValue::Bool(_)) => ConfigDomainValue::Bool(true),
      (Self::Choice(names), ConfigDomainValue::Choice(i)) => {
        ConfigDomainValue::Choice((i + 1).min(names.len().saturating_sub(1)))
      }
      _ => value,
    }
  }

  pub fn dec(&self, value: ConfigDomainValue) -> ConfigDomainValue {
    match (self, value) {
      (Self::Int { min, step, .. }, ConfigDomainValue::Int(v)) => {
        ConfigDomainValue::Int((v - step).max(*min))
      }
      (Self::Bool, ConfigDomainValue::Bool(_)) => ConfigDomainValue::Bool(false),
      (Self::Choice(_), ConfigDomainValue::Choice(i)) => {
        ConfigDomainValue::Choice(i.saturating_sub(1))
      }
      _ => value,
    }
  }

  /// Whether this domain can take the value, for values from outside the app such as the console
  pub(crate) fn contains(&self, value: ConfigDomainValue) -> bool {
    match (self, value) {
      (Self::Int { min, max, .. }, ConfigDomainValue::Int(v)) => (*min..=*max).contains(&v),
      (Self::Bool, ConfigDomainValue::Bool(_)) => true,
      (Self::Choice(names), ConfigDomainValue::Choice(i)) => i < names.len(),
      _ => false,
    }
  }

  /// Read a value saved in TOML, or `None` if it doesn't fit this domain. Ints outside the range are clamped.
  pub fn from_toml(&self, raw: &toml::Value) -> Option<ConfigDomainValue> {
    match (self, raw) {
      (Self::Int { min, max, .. }, toml::Value::Integer(v)) => {
        Some(ConfigDomainValue::Int((*v).clamp(*min, *max)))
      }
      (Self::Bool, toml::Value::Boolean(v)) => Some(ConfigDomainValue::Bool(*v)),
      (Self::Choice(names), toml::Value::String(name)) => names
        .iter()
        .position(|n| n == name)
        .map(ConfigDomainValue::Choice),
      _ => None,
    }
  }

  /// The value as it's saved in TOML. Choices are saved by name so reordering them doesn't change a saved value.
  pub fn to_toml(&self, value: ConfigDomainValue) -> toml::Value {
    match (self, value) {
      (Self::Choice(names), ConfigDomainValue::Choice(i)) => {
        toml::Value::String(names[i].to_string())
      }
      (_, ConfigDomainValue::Int(v)) => toml::Value::Integer(v),
      (_, ConfigDomainValue::Bool(v)) => toml::Value::Boolean(v),
      (_, ConfigDomainValue::Choice(i)) => toml::Value::Integer(i as i64),
    }
  }
}

/// Something that can describe the domain of a `T` config value
pub trait IntoConfigDomain<T> {
  fn into_domain(self) -> ConfigDomain;
}

#[cfg(test)]
mod tests {
  use super::*;

  const INT: ConfigDomain = ConfigDomain::Int {
    min: 0,
    max: 10,
    step: 4,
  };
  const CHOICE: ConfigDomain = ConfigDomain::Choice(&["free_play", "credit"]);

  #[test]
  fn int_steps_clamp_at_the_ends() {
    assert_eq!(
      INT.inc(ConfigDomainValue::Int(8)),
      ConfigDomainValue::Int(10)
    );
    assert_eq!(
      INT.dec(ConfigDomainValue::Int(2)),
      ConfigDomainValue::Int(0)
    );
    assert_eq!(
      INT.inc(ConfigDomainValue::Int(4)),
      ConfigDomainValue::Int(8)
    );
  }

  #[test]
  fn choice_steps_clamp_at_the_ends() {
    assert_eq!(
      CHOICE.inc(ConfigDomainValue::Choice(1)),
      ConfigDomainValue::Choice(1)
    );
    assert_eq!(
      CHOICE.dec(ConfigDomainValue::Choice(0)),
      ConfigDomainValue::Choice(0)
    );
    assert_eq!(
      CHOICE.inc(ConfigDomainValue::Choice(0)),
      ConfigDomainValue::Choice(1)
    );
  }

  #[test]
  fn choices_are_saved_by_name() {
    let saved = CHOICE.to_toml(ConfigDomainValue::Choice(1));
    assert_eq!(saved, toml::Value::String("credit".into()));
    assert_eq!(CHOICE.from_toml(&saved), Some(ConfigDomainValue::Choice(1)));
    assert_eq!(CHOICE.from_toml(&toml::Value::String("coins".into())), None);
  }

  #[test]
  fn contains_checks_the_range_and_the_kind_of_value() {
    assert!(INT.contains(ConfigDomainValue::Int(10)));
    assert!(!INT.contains(ConfigDomainValue::Int(11)));
    assert!(!INT.contains(ConfigDomainValue::Bool(true)));
    assert!(CHOICE.contains(ConfigDomainValue::Choice(1)));
    assert!(!CHOICE.contains(ConfigDomainValue::Choice(2)));
  }

  #[test]
  fn saved_ints_are_clamped_and_mismatches_rejected() {
    assert_eq!(
      INT.from_toml(&toml::Value::Integer(50)),
      Some(ConfigDomainValue::Int(10))
    );
    assert_eq!(INT.from_toml(&toml::Value::Boolean(true)), None);
  }
}
