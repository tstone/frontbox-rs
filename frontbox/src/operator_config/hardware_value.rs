use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::operator_config::generalized_config_value::GeneralizedConfigValue;
use crate::operator_config::*;

/// A value that can be assigned to configure a piece of hardware, either with a fixed value or using operator config
#[derive(Clone, Debug)]
pub enum HardwareValue<T: Clone, D: Domain<T> = Range<T>> {
  Config(ConfigValue<T, D>),
  Fixed(T),
}

impl<T, D: Domain<T>> HardwareValue<T, D>
where
  T: Clone + Send + Sync + 'static,
{
  pub fn fixed(value: T) -> Self {
    Self::Fixed(value)
  }

  pub fn config(name: &'static str, desc: &'static str, default: T, domain: D) -> Self {
    Self::Config(ConfigValue {
      name,
      desc,
      default,
      domain,
    })
  }

  /// Get the actual usable value given a working environment
  pub fn resolve(&self, op: &OperatorConfig) -> T {
    match self {
      Self::Fixed(v) => v.clone(),
      Self::Config(cv) => cv.resolve(op),
    }
  }

  /// Typed version for TOML read/write
  pub fn generalized_config_value(&self) -> Option<&dyn GeneralizedConfigValue>
  where
    T: PartialEq + ConfigDisplay + Serialize + DeserializeOwned + Send + Sync,
    D: Send + Sync,
  {
    match self {
      Self::Config(cv) => Some(cv),
      Self::Fixed(_) => None,
    }
  }
}

/// Allows plain values to be used anywhere a fixed HardwareValue is accepted
impl<T: Clone, D: Domain<T>> From<T> for HardwareValue<T, D> {
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
  name: &'static str,
  desc: &'static str,
  default: Repr<'a, T>,
}

impl<T: Clone + HardwareValueRepr, D: Domain<T>> Serialize for HardwareValue<T, D> {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    match self {
      Self::Config(cv) => serializer.serialize_newtype_variant(
        "HardwareValue",
        0,
        "Config",
        &ConfigRepr {
          name: cv.name,
          desc: cv.desc,
          default: Repr(&cv.default),
        },
      ),
      Self::Fixed(value) => {
        serializer.serialize_newtype_variant("HardwareValue", 1, "Fixed", &Repr(value))
      }
    }
  }
}

#[cfg(feature = "ts")]
impl<T, D> ts_rs::TS for HardwareValue<T, D>
where
  T: Clone + HardwareValueRepr + 'static,
  D: Domain<T> + 'static,
{
  type WithoutGenerics = Self;
  type OptionInnerType = Self;

  fn name(_: &ts_rs::Config) -> String {
    let value = T::ts_repr();
    format!(
      "{{ \"Config\": {{ name: string, desc: string, default: {value} }} }} | {{ \"Fixed\": {value} }}"
    )
  }

  fn inline(cfg: &ts_rs::Config) -> String {
    Self::name(cfg)
  }
}
