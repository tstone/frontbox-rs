use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use tokio::sync::mpsc;

use crate::operator_config::{Config, ConfigDefinition, ConfigDomainValue, ConfigType};
use crate::prelude::app_message::AppMessage;
use crate::prelude::*;

#[derive(Default)]
pub struct ConfigCollection {
  entries: RefCell<HashMap<&'static str, Entry>>,
  pending_disk: HashMap<String, toml::Value>, // raw until its key is first used
  pub(crate) app_sender: Option<mpsc::UnboundedSender<AppMessage>>,
}

impl ConfigCollection {
  /// Reads saved values from disk. Each is held raw until a config with its key is first used.
  pub fn load_from_disk(path: impl AsRef<Path>) -> Self {
    let pending_disk = fs::read_to_string(path)
      .ok()
      .and_then(|s| toml::from_str(&s).ok())
      .unwrap_or_default();
    Self {
      entries: RefCell::default(),
      pending_disk,
      app_sender: None,
    }
  }

  pub fn get<T: ConfigType>(&self, config: &Config<T>) -> T {
    T::from_value(self.inspect(config.display()))
  }

  pub fn set<T: ConfigType>(&self, config: &Config<T>, value: T) {
    self.set_value(config.display(), value.to_value());
  }

  /// The current value without its Rust type, for listing configs of mixed types
  pub fn inspect(&self, config: &ConfigDefinition) -> ConfigDomainValue {
    self.with_entry(config, |entry| entry.value)
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
    self.with_entry(config, |entry| entry.value = value);

    if let Some(app_sender) = &self.app_sender {
      let event = ConfigChanged(config.name);
      let _ = app_sender.send(AppMessage::EmitEvent(None, EventBox::new(event)));
    }
  }

  /// Runs `f` on the entry for this config, creating it on first use. `f` must not call back into the store.
  fn with_entry<R>(&self, config: &ConfigDefinition, f: impl FnOnce(&mut Entry) -> R) -> R {
    let mut entries = self.entries.borrow_mut();
    let entry = entries.entry(config.name).or_insert_with(|| Entry {
      definition: config.clone(),
      value: self.saved_or_default(config),
    });
    f(entry)
  }

  fn saved_or_default(&self, config: &ConfigDefinition) -> ConfigDomainValue {
    match self.pending_disk.get(config.name) {
      Some(raw) => config.domain.from_toml(raw).unwrap_or_else(|| {
        log::warn!(
          "config '{}': saved value {raw} doesn't fit, using default",
          config.name
        );
        config.default
      }),
      None => config.default,
    }
  }
}

struct Entry {
  /// The first declaration seen for this key
  #[allow(dead_code)] // read once saving and the conflict check land
  definition: ConfigDefinition,
  value: ConfigDomainValue,
}

#[derive(serde::Serialize, Event)]
pub struct ConfigChanged(pub &'static str);

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use super::*;
  use crate::operator_config::Ranges;

  #[test]
  fn saved_values_are_read_without_registration() {
    let pulse = Config::new(
      "pulse",
      "",
      Duration::from_millis(20),
      Ranges::duration(5, 100),
    );
    let power = Config::new("power", "", Power::HALF, Ranges::full_power());

    let config = saved("pulse = 35\npower = 200");

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
  fn values_without_a_saved_entry_read_as_default() {
    let turns = Config::new("turns", "", 3u8, Ranges::u8(1, 5));
    assert_eq!(saved("other = 1").get(&turns), 3);
  }

  #[test]
  fn saved_values_outside_the_domain_are_clamped_or_defaulted() {
    let turns = Config::new("turns", "", 3u8, Ranges::u8(1, 5));
    assert_eq!(saved("turns = 9").get(&turns), 5);
    assert_eq!(saved("turns = \"lots\"").get(&turns), 3);
  }

  #[test]
  fn stepping_starts_from_the_saved_value() {
    let turns = Config::new("turns", "", 3u8, Ranges::u8(1, 5));
    let config = saved("turns = 1");
    config.inc(turns.display());
    assert_eq!(config.get(&turns), 2);
  }

  fn saved(toml: &str) -> ConfigCollection {
    ConfigCollection {
      pending_disk: toml::from_str(toml).unwrap(),
      ..Default::default()
    }
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
