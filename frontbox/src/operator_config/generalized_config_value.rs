use std::any::Any;
use std::collections::HashMap;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::operator_config::*;
use crate::prelude::*;

/// A wrapper used to retain type and read/write operator config values from TOML
pub trait GeneralizedConfigValue: Send + Sync {
  // read/write persistence
  fn insert_default(&self, store: &mut HashMap<&'static str, Box<dyn Any + Send + Sync>>);
  fn load_from_toml(
    &self,
    raw: &toml::Value,
    store: &mut HashMap<&'static str, Box<dyn Any + Send + Sync>>,
  );
  fn save_to_toml(&self, store: &HashMap<&'static str, Box<dyn Any + Send + Sync>>) -> toml::Value;
  // display rendering
  fn text(&self) -> &'static str;
  fn description(&self) -> &'static str;
  fn current_value(&self, ctx: &Context) -> String;
  /// True if the value is NOT default
  fn value_modified(&self, ctx: &Context) -> bool;
  fn increment(&self, ctx: &Context) -> String;
  fn decrement(&self, ctx: &Context) -> String;
}

impl<T, D> HardwareValue<T, D>
where
  T: PartialEq + Clone + 'static,
  D: Domain<T>,
{
  pub fn config_value(&self) -> Option<&dyn GeneralizedConfigValue>
  where
    T: ConfigDisplay + Serialize + DeserializeOwned + Send + Sync,
    D: Send + Sync,
  {
    match self {
      Self::Config(cv) => Some(cv),
      Self::Fixed(_) => None,
    }
  }
}

impl<T, D> GeneralizedConfigValue for ConfigValue<T, D>
where
  T: ConfigDisplay + Clone + Serialize + DeserializeOwned + Send + Sync + PartialEq + 'static,
  D: Domain<T> + Send + Sync,
{
  fn text(&self) -> &'static str {
    self.name
  }

  fn description(&self) -> &'static str {
    self.desc
  }

  fn current_value(&self, ctx: &Context) -> String {
    let op_config = ctx
      .systems
      .get::<OperatorConfig>()
      .expect("Operator Config system not running");
    let value = op_config.get(self);
    format!("{}", value.config_display())
  }

  fn value_modified(&self, ctx: &Context) -> bool {
    let op_config = ctx
      .systems
      .get::<OperatorConfig>()
      .expect("Operator Config system not running");
    op_config.get(self) != self.default
  }

  fn decrement(&self, ctx: &Context) -> String {
    let mut op_config = ctx
      .systems
      .get::<OperatorConfig>()
      .expect("Operator Config system not running");

    let old_value = op_config.get(self);
    let next_value = self.domain.dec(&old_value);
    let text = format!("{}", next_value.config_display());
    op_config.set(self, next_value);

    text
  }

  fn increment(&self, ctx: &Context) -> String {
    let mut op_config = ctx
      .systems
      .get::<OperatorConfig>()
      .expect("Operator Config system not running");

    let old_value = op_config.get(self);
    let next_value = self.domain.inc(&old_value);
    let text = format!("{}", next_value.config_display());
    op_config.set(self, next_value);

    text
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
