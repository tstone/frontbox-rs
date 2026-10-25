use std::collections::HashMap;
use std::fs;
use std::path::Path;

use tokio::sync::mpsc;

use crate::operator_config::{ConfigDisplay, ConfigType, ConfigValue, RawConfigValue};
use crate::prelude::app_message::AppMessage;
use crate::prelude::*;

#[derive(Default)]
pub struct OperatorConfig {
  current_values: HashMap<&'static str, RawConfigValue>,
  pending_disk: HashMap<String, toml::Value>, // raw until a matching ConfigValue registers
  pub(crate) app_sender: Option<mpsc::UnboundedSender<AppMessage>>,
}

impl OperatorConfig {
  /// Reads values from disk into a temporary buffer, but waits until a config value is registered before assignment
  pub fn load_from_disk(path: impl AsRef<Path>) -> Self {
    let pending_disk = fs::read_to_string(path)
      .ok()
      .and_then(|s| toml::from_str(&s).ok())
      .unwrap_or_default();
    Self {
      current_values: HashMap::new(),
      pending_disk,
      app_sender: None,
    }
  }

  /// Activate a config value and automatically pre-populate it with either the previously saved value or the default
  pub fn register(&mut self, cv: &ConfigDisplay) {
    if self.current_values.contains_key(cv.name) {
      return; // already registered — safe if the same ConfigValue is reachable from two defs
    }
    let value = match self.pending_disk.get(cv.name) {
      Some(raw) => cv.domain.from_toml(raw).unwrap_or_else(|| {
        log::warn!(
          "config '{}': saved value {raw} doesn't fit, using default",
          cv.name
        );
        cv.default
      }),
      None => cv.default,
    };
    self.current_values.insert(cv.name, value);
  }

  pub fn get<T: ConfigType>(&self, config: &ConfigValue<T>) -> T {
    T::from_raw(self.get_raw(config.display()))
  }

  pub fn set<T: ConfigType>(&mut self, config: &ConfigValue<T>, value: T) {
    self.set_raw(config.display(), value.to_raw());
  }

  pub fn get_raw(&self, config: &ConfigDisplay) -> RawConfigValue {
    self
      .current_values
      .get(config.name)
      .copied()
      .unwrap_or(config.default)
  }

  pub fn set_raw(&mut self, config: &ConfigDisplay, value: RawConfigValue) {
    self.current_values.insert(config.name, value);

    if let Some(app_sender) = &self.app_sender {
      let event = OperatorConfigChanged(config.name);
      let _ = app_sender.send(AppMessage::EmitEvent(None, EventBox::new(event)));
    }
  }
}

impl System for OperatorConfig {}

#[derive(serde::Serialize, Event)]
pub struct OperatorConfigChanged(pub &'static str);

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use super::*;
  use crate::operator_config::Ranges;

  #[test]
  fn typed_values_round_trip_through_saved_toml() {
    let pulse = ConfigValue::new(
      "pulse",
      "",
      Duration::from_millis(20),
      Ranges::duration(5, 100),
    );
    let power = ConfigValue::new("power", "", Power::HALF, Ranges::full_power());

    let mut op_config = OperatorConfig::default();
    op_config.pending_disk = toml::from_str("pulse = 35\npower = 200").unwrap();
    op_config.register(pulse.display());
    op_config.register(power.display());

    assert_eq!(op_config.get(&pulse), Duration::from_millis(35));
    assert_eq!(op_config.get(&power), Power { power: 200 });
    assert_eq!(
      pulse.display().format(op_config.get_raw(pulse.display())),
      "35ms"
    );
    assert_eq!(
      power.display().format(op_config.get_raw(power.display())),
      "78%"
    );
  }

  #[test]
  fn unregistered_values_read_as_default() {
    let turns = ConfigValue::new("turns", "", 3u8, Ranges::u8(1, 5));
    assert_eq!(OperatorConfig::default().get(&turns), 3);
  }
}
