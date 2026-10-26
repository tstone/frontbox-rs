use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use tokio::sync::mpsc;

use crate::operator_config::{Config, ConfigDefinition, ConfigDomainValue, ConfigType};
use crate::prelude::app_message::AppMessage;
use crate::prelude::*;

/// The live operator config values. Reads and writes go through `&self`, each borrowing only for its own call, so
/// systems can use it freely through `ctx.config`.
#[derive(Default)]
pub struct ConfigCollection {
  current_values: RefCell<HashMap<&'static str, ConfigDomainValue>>,
  pending_disk: HashMap<String, toml::Value>, // raw until a matching Config registers
  pub(crate) app_sender: Option<mpsc::UnboundedSender<AppMessage>>,
}

impl ConfigCollection {
  /// Reads values from disk into a temporary buffer, but waits until a config value is registered before assignment
  pub fn load_from_disk(path: impl AsRef<Path>) -> Self {
    let pending_disk = fs::read_to_string(path)
      .ok()
      .and_then(|s| toml::from_str(&s).ok())
      .unwrap_or_default();
    Self {
      current_values: RefCell::default(),
      pending_disk,
      app_sender: None,
    }
  }

  /// Activate a config value and automatically pre-populate it with either the previously saved value or the default
  pub fn register(&self, config: &ConfigDefinition) {
    let mut current_values = self.current_values.borrow_mut();
    if current_values.contains_key(config.name) {
      return; // already registered — safe if the same Config is reachable from two defs
    }
    let value = match self.pending_disk.get(config.name) {
      Some(raw) => config.domain.from_toml(raw).unwrap_or_else(|| {
        log::warn!(
          "config '{}': saved value {raw} doesn't fit, using default",
          config.name
        );
        config.default
      }),
      None => config.default,
    };
    current_values.insert(config.name, value);
  }

  pub fn get<T: ConfigType>(&self, config: &Config<T>) -> T {
    T::from_value(self.inspect(config.display()))
  }

  pub fn set<T: ConfigType>(&self, config: &Config<T>, value: T) {
    self.set_value(config.display(), value.to_value());
  }

  /// The current value without its Rust type, for listing configs of mixed types
  pub fn inspect(&self, config: &ConfigDefinition) -> ConfigDomainValue {
    self
      .current_values
      .borrow()
      .get(config.name)
      .copied()
      .unwrap_or(config.default)
  }

  /// Step the value up within its domain, returning the new value
  pub fn inc(&self, config: &ConfigDefinition) -> ConfigDomainValue {
    let next = config.domain.inc(self.inspect(config));
    self.set_value(config, next);
    next
  }

  /// Step the value down within its domain, returning the new value
  pub fn dec(&self, config: &ConfigDefinition) -> ConfigDomainValue {
    let next = config.domain.dec(self.inspect(config));
    self.set_value(config, next);
    next
  }

  fn set_value(&self, config: &ConfigDefinition, value: ConfigDomainValue) {
    self.current_values.borrow_mut().insert(config.name, value);

    if let Some(app_sender) = &self.app_sender {
      let event = ConfigChanged(config.name);
      let _ = app_sender.send(AppMessage::EmitEvent(None, EventBox::new(event)));
    }
  }
}

#[derive(serde::Serialize, Event)]
pub struct ConfigChanged(pub &'static str);

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use super::*;
  use crate::operator_config::Ranges;

  #[test]
  fn typed_values_round_trip_through_saved_toml() {
    let pulse = Config::new(
      "pulse",
      "",
      Duration::from_millis(20),
      Ranges::duration(5, 100),
    );
    let power = Config::new("power", "", Power::HALF, Ranges::full_power());

    let mut config = ConfigCollection::default();
    config.pending_disk = toml::from_str("pulse = 35\npower = 200").unwrap();
    config.register(pulse.display());
    config.register(power.display());

    assert_eq!(config.get(&pulse), Duration::from_millis(35));
    assert_eq!(config.get(&power), Power { power: 200 });
    assert_eq!(
      pulse.display().format(config.inspect(pulse.display())),
      "35ms"
    );
    assert_eq!(
      power.display().format(config.inspect(power.display())),
      "78%"
    );
  }

  #[test]
  fn unregistered_values_read_as_default() {
    let turns = Config::new("turns", "", 3u8, Ranges::u8(1, 5));
    assert_eq!(ConfigCollection::default().get(&turns), 3);
  }

  #[test]
  fn set_through_context_reads_back_and_emits_changed() {
    let turns = Config::new("turns", "", 3u8, Ranges::u8(1, 5));
    let mut test_ctx = TestContext::default();

    let ctx = test_ctx.sys_ctx();
    ctx.config.set(&turns, 4);
    assert_eq!(turns.get(&ctx), 4);

    let changed: Vec<&str> = test_ctx
      .events_emitted()
      .iter()
      .filter_map(|e| e.event.downcast_ref::<ConfigChanged>())
      .map(|e| e.0)
      .collect();
    assert_eq!(changed, vec!["turns"]);
  }

  #[test]
  fn inc_and_dec_through_context_emit_changed() {
    let turns = Config::new("turns", "", 3u8, Ranges::u8(1, 5));
    let mut test_ctx = TestContext::default();

    let ctx = test_ctx.svc_ctx();
    ctx.config.inc(turns.display());
    ctx.config.inc(turns.display());
    ctx.config.dec(turns.display());
    assert_eq!(ctx.config.get(&turns), 4);

    let changed = test_ctx
      .events_emitted()
      .iter()
      .filter(|e| e.event.is::<ConfigChanged>())
      .count();
    assert_eq!(changed, 3);
  }
}
