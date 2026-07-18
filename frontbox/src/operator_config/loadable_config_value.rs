use std::any::Any;
use std::collections::HashMap;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::operator_config::*;

/// A wrapper used to retain type and read/write operator config values from TOML
pub trait LoadableConfigValue: Send + Sync {
  fn key(&self) -> &'static str;
  fn insert_default(&self, store: &mut HashMap<&'static str, Box<dyn Any + Send + Sync>>);
  fn load_from_toml(
    &self,
    raw: &toml::Value,
    store: &mut HashMap<&'static str, Box<dyn Any + Send + Sync>>,
  );
  fn save_to_toml(&self, store: &HashMap<&'static str, Box<dyn Any + Send + Sync>>) -> toml::Value;
}

impl<T, D> HardwareValue<T, D>
where
  T: Clone + 'static,
  D: Domain<T>,
{
  pub fn config_value(&self) -> Option<&dyn LoadableConfigValue>
  where
    T: Serialize + DeserializeOwned + Send + Sync,
    D: Send + Sync,
  {
    match self {
      Self::Config(cv) => Some(cv),
      Self::Fixed(_) => None,
    }
  }
}

impl<T, D> LoadableConfigValue for ConfigValue<T, D>
where
  T: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
  D: Domain<T> + Send + Sync,
{
  fn key(&self) -> &'static str {
    self.name
  }

  fn insert_default(&self, store: &mut HashMap<&'static str, Box<dyn Any + Send + Sync>>) {
    store.insert(self.name, Box::new(self.default.clone()));
  }

  fn load_from_toml(
    &self,
    raw: &toml::Value,
    store: &mut HashMap<&'static str, Box<dyn Any + Send + Sync>>,
  ) {
    match raw.clone().try_into::<T>() {
      Ok(v) => {
        store.insert(self.name, Box::new(v));
      }
      Err(e) => eprintln!(
        "config '{}': failed to parse, using default ({e})",
        self.name
      ),
    }
  }

  fn save_to_toml(&self, store: &HashMap<&'static str, Box<dyn Any + Send + Sync>>) -> toml::Value {
    let value = store
      .get(self.name)
      .and_then(|v| v.downcast_ref::<T>())
      .unwrap_or(&self.default);
    toml::Value::try_from(value.clone()).expect("config value not serializable")
  }
}
