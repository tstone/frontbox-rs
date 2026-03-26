use std::collections::HashMap;

use crate::prelude::*;

pub type OperatorConfigStore = HashMap<&'static str, ConfigItem>;

#[derive(Debug)]
pub struct OperatorConfig {
  internal: OperatorConfigStore,
}

impl OperatorConfig {
  pub fn new(store: OperatorConfigStore) -> Self {
    Self { internal: store }
  }

  pub fn set_value(&mut self, key: &'static str, value: impl Into<ConfigValue>, ctx: &mut Context) {
    let value = value.into();
    if let Some(item) = self.internal.get_mut(key) {
      let old_value = item.value();
      let new_value = value.clone();
      match (item, value) {
        (ConfigItem::String { current, .. }, ConfigValue::String(v)) => *current = v,
        (ConfigItem::Integer { value: current, .. }, ConfigValue::Integer(v)) => *current = v,
        (ConfigItem::Boolean { current, .. }, ConfigValue::Boolean(v)) => *current = v,
        _ => {}
      }
      ctx.emit(ConfigChanged::new(key, old_value, new_value));
    }
  }

  fn get(&self, key: &'static str) -> Option<&ConfigItem> {
    self.internal.get(key)
  }

  #[allow(unused)]
  fn get_mut(&mut self, key: &'static str) -> Option<&mut ConfigItem> {
    self.internal.get_mut(key)
  }

  pub fn get_string(&self, key: &'static str) -> Option<String> {
    self.get(key).and_then(|item| match item {
      ConfigItem::String { current, .. } => Some(current.clone()),
      _ => None,
    })
  }

  pub fn get_integer(&self, key: &'static str) -> Option<i32> {
    self.get(key).and_then(|item| match item {
      ConfigItem::Integer { value, .. } => Some(*value),
      _ => None,
    })
  }

  pub fn get_boolean(&self, key: &'static str) -> Option<bool> {
    self.get(key).and_then(|item| match item {
      ConfigItem::Boolean { current, .. } => Some(*current),
      _ => None,
    })
  }
}

impl System for OperatorConfig {}

pub struct ConfigChanged {
  pub key: &'static str,
  pub old_value: ConfigValue,
  pub new_value: ConfigValue,
}

impl ConfigChanged {
  pub fn new(key: &'static str, old_value: ConfigValue, new_value: ConfigValue) -> Self {
    Self {
      key,
      old_value,
      new_value,
    }
  }
}
