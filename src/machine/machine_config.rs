use std::collections::HashMap;

use crate::machine::config_value::{ConfigItem, ConfigValue};

pub struct MachineConfig {
  internal: HashMap<&'static str, ConfigItem>,
  change_queue: Vec<&'static str>,
}

impl MachineConfig {
  pub fn new() -> Self {
    Self {
      internal: HashMap::new(),
      change_queue: Vec::new(),
    }
  }

  pub fn add_item(&mut self, key: &'static str, item: ConfigItem) {
    self.internal.insert(key, item);
  }

  pub fn set_value(&mut self, key: &'static str, value: impl Into<ConfigValue>) {
    let value = value.into();
    self.change_queue.push(key);
    if let Some(item) = self.internal.get_mut(key) {
      match (item, value) {
        (ConfigItem::String { current, .. }, ConfigValue::String(v)) => *current = v,
        (ConfigItem::Integer { current, .. }, ConfigValue::Integer(v)) => *current = v,
        (ConfigItem::Boolean { current, .. }, ConfigValue::Boolean(v)) => *current = v,
        _ => {}
      }
    }
  }

  pub fn get_item(&self, key: &'static str) -> Option<&ConfigItem> {
    self.internal.get(key)
  }

  pub fn get_value(&self, key: &'static str) -> Option<ConfigValue> {
    self.internal.get(key).map(|item| match item {
      ConfigItem::String { current, .. } => ConfigValue::String(current.clone()),
      ConfigItem::Integer { current, .. } => ConfigValue::Integer(*current),
      ConfigItem::Boolean { current, .. } => ConfigValue::Boolean(*current),
    })
  }

  pub fn get_value_as_string(&self, key: &'static str) -> Option<String> {
    self.get_value(key).and_then(|v| v.as_string())
  }

  pub fn get_value_as_integer(&self, key: &'static str) -> Option<i32> {
    self.get_value(key).and_then(|v| v.as_integer())
  }

  pub fn get_value_as_boolean(&self, key: &'static str) -> Option<bool> {
    self.get_value(key).and_then(|v| v.as_boolean())
  }

  pub fn get_value_as_usize(&self, key: &'static str) -> Option<usize> {
    self.get_value(key).and_then(|v| v.as_usize())
  }

  pub fn get_value_as_u64(&self, key: &'static str) -> Option<u64> {
    self.get_value_as_integer(key).map(|v| v as u64)
  }

  pub fn get_value_as_u32(&self, key: &'static str) -> Option<u32> {
    self.get_value(key).and_then(|v| v.as_u32())
  }

  pub fn get_value_as_u16(&self, key: &'static str) -> Option<u16> {
    self.get_value(key).and_then(|v| v.as_u16())
  }

  pub fn get_value_as_u8(&self, key: &'static str) -> Option<u8> {
    self.get_value(key).and_then(|v| v.as_u8())
  }

  pub fn read_changes(&mut self) -> Option<&'static str> {
    self.change_queue.pop()
  }
}

impl Default for MachineConfig {
  fn default() -> Self {
    let mut config = Self::new();

    config.add_item(
      "watchdog.tick_ms",
      ConfigItem::Integer {
        current: 1000,
        min: 100,
        max: 5000,
        default: 1000,
        name: "Watchdog Tick (ms)",
        description: "The interval in milliseconds between each watchdog timer tick.",
      },
    );

    config.add_item(
      "system.timer_tick_ms",
      ConfigItem::Integer {
        current: 50,
        min: 1,
        max: 5000,
        default: 50,
        name: "System Timer Tick (ms)",
        description: "Resolution of the system timers. Lower values allow for more precise timers but may increase CPU usage.",
      },
    );

    config
  }
}
