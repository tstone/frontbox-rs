/// A config value in the form the menu, persistence, and console work with, independent of its Rust type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawConfigValue {
  Int(i64),
  Bool(bool),
  /// Index into the domain's choices
  Choice(usize),
}

/// The values a config is allowed to take. Stepping clamps at the ends rather than wrapping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigDomain {
  Int {
    min: i64,
    max: i64,
    step: i64,
  },
  Bool,
  /// Choice names, which are also how a choice is saved
  Choice(&'static [&'static str]),
}

impl ConfigDomain {
  pub fn inc(&self, value: RawConfigValue) -> RawConfigValue {
    match (self, value) {
      (Self::Int { max, step, .. }, RawConfigValue::Int(v)) => {
        RawConfigValue::Int((v + step).min(*max))
      }
      (Self::Bool, RawConfigValue::Bool(_)) => RawConfigValue::Bool(true),
      (Self::Choice(names), RawConfigValue::Choice(i)) => {
        RawConfigValue::Choice((i + 1).min(names.len().saturating_sub(1)))
      }
      _ => value,
    }
  }

  pub fn dec(&self, value: RawConfigValue) -> RawConfigValue {
    match (self, value) {
      (Self::Int { min, step, .. }, RawConfigValue::Int(v)) => {
        RawConfigValue::Int((v - step).max(*min))
      }
      (Self::Bool, RawConfigValue::Bool(_)) => RawConfigValue::Bool(false),
      (Self::Choice(_), RawConfigValue::Choice(i)) => RawConfigValue::Choice(i.saturating_sub(1)),
      _ => value,
    }
  }

  /// Read a value saved in TOML, or `None` if it doesn't fit this domain. Ints outside the range are clamped.
  pub fn from_toml(&self, raw: &toml::Value) -> Option<RawConfigValue> {
    match (self, raw) {
      (Self::Int { min, max, .. }, toml::Value::Integer(v)) => {
        Some(RawConfigValue::Int((*v).clamp(*min, *max)))
      }
      (Self::Bool, toml::Value::Boolean(v)) => Some(RawConfigValue::Bool(*v)),
      (Self::Choice(names), toml::Value::String(name)) => names
        .iter()
        .position(|n| n == name)
        .map(RawConfigValue::Choice),
      _ => None,
    }
  }

  /// The value as it's saved in TOML. Choices are saved by name so reordering them doesn't change a saved value.
  pub fn to_toml(&self, value: RawConfigValue) -> toml::Value {
    match (self, value) {
      (Self::Choice(names), RawConfigValue::Choice(i)) => toml::Value::String(names[i].to_string()),
      (_, RawConfigValue::Int(v)) => toml::Value::Integer(v),
      (_, RawConfigValue::Bool(v)) => toml::Value::Boolean(v),
      (_, RawConfigValue::Choice(i)) => toml::Value::Integer(i as i64),
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
    assert_eq!(INT.inc(RawConfigValue::Int(8)), RawConfigValue::Int(10));
    assert_eq!(INT.dec(RawConfigValue::Int(2)), RawConfigValue::Int(0));
    assert_eq!(INT.inc(RawConfigValue::Int(4)), RawConfigValue::Int(8));
  }

  #[test]
  fn choice_steps_clamp_at_the_ends() {
    assert_eq!(
      CHOICE.inc(RawConfigValue::Choice(1)),
      RawConfigValue::Choice(1)
    );
    assert_eq!(
      CHOICE.dec(RawConfigValue::Choice(0)),
      RawConfigValue::Choice(0)
    );
    assert_eq!(
      CHOICE.inc(RawConfigValue::Choice(0)),
      RawConfigValue::Choice(1)
    );
  }

  #[test]
  fn choices_are_saved_by_name() {
    let saved = CHOICE.to_toml(RawConfigValue::Choice(1));
    assert_eq!(saved, toml::Value::String("credit".into()));
    assert_eq!(CHOICE.from_toml(&saved), Some(RawConfigValue::Choice(1)));
    assert_eq!(CHOICE.from_toml(&toml::Value::String("coins".into())), None);
  }

  #[test]
  fn saved_ints_are_clamped_and_mismatches_rejected() {
    assert_eq!(
      INT.from_toml(&toml::Value::Integer(50)),
      Some(RawConfigValue::Int(10))
    );
    assert_eq!(INT.from_toml(&toml::Value::Boolean(true)), None);
  }
}
