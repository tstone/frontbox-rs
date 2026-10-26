use serde::Serialize;

use crate::operator_config::*;
use crate::prelude::BootSnapshot;

/// A value that can be assigned to configure a piece of hardware, either with a fixed value or using operator config
#[derive(Clone, Debug)]
pub enum HardwareValue<T: Clone> {
  Config(Config<T>),
  Fixed(T),
}

impl<T: Clone + ConfigType> HardwareValue<T> {
  pub fn fixed(value: T) -> Self {
    Self::Fixed(value)
  }

  /// The key defaults to the label; see [`HardwareValue::key`]
  pub fn config(
    label: &'static str,
    desc: &'static str,
    default: T,
    domain: impl IntoConfigDomain<T>,
  ) -> Self {
    Self::Config(Config::new(label, desc, default, domain))
  }

  /// Set the config's `key`. Key must be unique but is derived from label by default. This force-sets it to a specific value for uniqueness.
  pub fn key(self, key: &'static str) -> Self {
    match self {
      Self::Config(config) => Self::Config(config.key(key)),
      Self::Fixed(_) => self,
    }
  }

  /// Get the actual usable value given a working environment
  pub fn resolve(&self, snap: &BootSnapshot) -> T {
    match self {
      Self::Fixed(v) => v.clone(),
      Self::Config(config) => snap.config.get(config),
    }
  }

  /// The config declaration, if this value is configurable
  pub fn config_display(&self) -> Option<&ConfigDefinition> {
    match self {
      Self::Config(config) => Some(config.display()),
      Self::Fixed(_) => None,
    }
  }
}

/// Allows plain values to be used anywhere a fixed HardwareValue is accepted
impl<T: Clone> From<T> for HardwareValue<T> {
  fn from(value: T) -> Self {
    Self::Fixed(value)
  }
}

/// How the value inside a [`HardwareValue`] is serialized, e.g. for tracers. Durations are
/// milliseconds, matching `SwitchConfig`.
pub trait HardwareValueRepr {
  fn serialize_repr<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>;

  /// TypeScript type of the serialized value
  #[cfg(feature = "ts")]
  fn ts_repr() -> &'static str;
}

impl HardwareValueRepr for std::time::Duration {
  fn serialize_repr<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_u64(self.as_millis() as u64)
  }

  #[cfg(feature = "ts")]
  fn ts_repr() -> &'static str {
    "number"
  }
}

impl HardwareValueRepr for fast_protocol::Power {
  fn serialize_repr<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    self.serialize(serializer)
  }

  #[cfg(feature = "ts")]
  fn ts_repr() -> &'static str {
    "{ power: number }"
  }
}

struct Repr<'a, T>(&'a T);

impl<T: HardwareValueRepr> Serialize for Repr<'_, T> {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    self.0.serialize_repr(serializer)
  }
}

/// The operator config backing a value; the domain is left out
#[derive(Serialize)]
#[serde(bound = "T: HardwareValueRepr")]
struct ConfigRepr<'a, T: HardwareValueRepr> {
  key: &'static str,
  label: &'static str,
  desc: &'static str,
  default: Repr<'a, T>,
}

impl<T: Clone + ConfigType + HardwareValueRepr> Serialize for HardwareValue<T> {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    match self {
      Self::Config(config) => serializer.serialize_newtype_variant(
        "HardwareValue",
        0,
        "Config",
        &ConfigRepr {
          key: config.display().key,
          label: config.display().label,
          desc: config.display().desc,
          default: Repr(&config.default()),
        },
      ),
      Self::Fixed(value) => {
        serializer.serialize_newtype_variant("HardwareValue", 1, "Fixed", &Repr(value))
      }
    }
  }
}

#[cfg(feature = "ts")]
impl<T> ts_rs::TS for HardwareValue<T>
where
  T: Clone + HardwareValueRepr + 'static,
{
  type WithoutGenerics = Self;
  type OptionInnerType = Self;

  fn name(_: &ts_rs::Config) -> String {
    let value = T::ts_repr();
    format!(
      "{{ \"Config\": {{ key: string, label: string, desc: string, default: {value} }} }} | {{ \"Fixed\": {value} }}"
    )
  }

  fn inline(cfg: &ts_rs::Config) -> String {
    Self::name(cfg)
  }
}
