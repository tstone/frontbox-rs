use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tokio::sync::{mpsc, watch};

use crate::operator_config::{Config, ConfigDefinition, ConfigDomainValue, ConfigType};
use crate::prelude::app_message::AppMessage;
use crate::prelude::*;

#[derive(Default)]
pub struct ConfigCollection {
  entries: RefCell<HashMap<&'static str, Entry>>,
  /// Release builds only: definitions that conflict with the first one seen for their key
  isolated: RefCell<Vec<Entry>>,
  pending_disk: toml::Table, // raw until its key is first used
  pub(crate) app_sender: Option<mpsc::UnboundedSender<AppMessage>>,
  /// Hands the latest table to the task writing it to disk, when there's a file to save to
  saver: Option<watch::Sender<toml::Table>>,
}

impl ConfigCollection {
  /// Reads saved values from disk. Each is held raw until a config with its key is first used.
  pub fn load_from_disk(path: impl AsRef<Path>) -> Self {
    Self {
      pending_disk: crate::toml::load(path).unwrap_or_default(),
      ..Default::default()
    }
  }

  /// Updates to config (I/O) should happen outside of the main run_loop, so they are spun off in a tokio task
  /// which watches changes to config and writes them to disk on change
  pub(crate) fn begin_save_task(&mut self, path: PathBuf) {
    let (tx, mut rx) = watch::channel(self.to_table());
    rx.mark_unchanged(); // nothing has changed since loading
    tokio::spawn(async move {
      // one save at a time; changes made during a save are coalesced into the next
      while rx.changed().await.is_ok() {
        let table = rx.borrow_and_update().clone();
        let path = path.clone();
        let saved = tokio::task::spawn_blocking(move || crate::toml::save(&path, &table)).await;
        if let Ok(Err(e)) = saved {
          log::error!("Failed to save operator config: {e}");
        }
      }
    });
    self.saver = Some(tx);
  }

  /// Save the current values (write happens in background task)
  pub(crate) fn save(&self) {
    if let Some(saver) = &self.saver {
      saver.send_replace(self.to_table());
    }
  }

  /// What's actually written to disk
  fn to_table(&self) -> toml::Table {
    let mut table = self.pending_disk.clone();
    for (key, entry) in self.entries.borrow().iter() {
      if entry.definition.is_default(entry.value) {
        table.remove(*key);
      } else {
        table.insert(
          key.to_string(),
          entry.definition.domain.to_toml(entry.value),
        );
      }
    }
    table
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
    let changed = self.with_entry(config, |entry| {
      let changed = entry.value != value;
      entry.value = value;
      changed
    });

    if changed && let Some(app_sender) = &self.app_sender {
      let _ = app_sender.send(AppMessage::ConfigChanged(config.key));
    }
  }

  /// Runs `f` on the entry for this config, creating it on first use. `f` must not call back into the store.
  fn with_entry<R>(&self, config: &ConfigDefinition, f: impl FnOnce(&mut Entry) -> R) -> R {
    let mut entries = self.entries.borrow_mut();
    let entry = entries.entry(config.key).or_insert_with(|| Entry {
      definition: config.clone(),
      value: self.saved_or_default(config),
    });
    if entry.definition.is_compatible(config) {
      return f(entry);
    }
    let first_label = entry.definition.label;
    drop(entries);

    // A conflicting definition gets a value of its own, so neither reads the other's value as the wrong type or range
    let mut isolated = self.isolated.borrow_mut();
    let index = match isolated
      .iter()
      .position(|e| e.definition.key == config.key && e.definition.is_compatible(config))
    {
      Some(index) => index,
      None => {
        let message = format!(
          "config key '{}' is declared by '{}' and '{}' with a different type, domain or default",
          config.key, first_label, config.label
        );
        if cfg!(debug_assertions) {
          panic!("{message}. Give one a distinct `.key(...)`");
        }
        log::error!(
          "{message}; '{}' uses its own default until fixed",
          config.label
        );
        isolated.push(Entry {
          definition: config.clone(),
          value: config.default,
        });
        isolated.len() - 1
      }
    };
    f(&mut isolated[index])
  }

  fn saved_or_default(&self, config: &ConfigDefinition) -> ConfigDomainValue {
    match self.pending_disk.get(config.key) {
      Some(raw) => config.domain.from_toml(raw).unwrap_or_else(|| {
        log::warn!(
          "config '{}': saved value {raw} doesn't fit, using default",
          config.key
        );
        config.default
      }),
      None => config.default,
    }
  }
}

struct Entry {
  /// The first declaration seen for this key
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

  #[test]
  fn key_defaults_to_label_and_can_be_overridden() {
    let turns = Config::new("Turns", "", 3u8, Ranges::u8(1, 5));
    let reworded = Config::new("Balls per Game", "", 3u8, Ranges::u8(1, 5)).key("Turns");
    assert_eq!(turns.display().key, "Turns");

    let config = saved("Turns = 4");
    assert_eq!(config.get(&turns), 4);
    assert_eq!(config.get(&reworded), 4);
  }

  #[test]
  fn identical_definitions_on_one_key_share_a_value() {
    let first = Config::new("turns", "", 3u8, Ranges::u8(1, 5));
    let second = Config::new("turns", "Another reader", 3u8, Ranges::u8(1, 5));
    let config = ConfigCollection::default();

    config.set(&first, 5);
    assert_eq!(config.get(&second), 5);
  }

  #[test]
  #[cfg(debug_assertions)]
  #[should_panic(expected = "different type, domain or default")]
  fn different_types_on_one_key_conflict() {
    // both are stored as Int, so without the type check a u8 3 would read as 3ms
    let count = Config::new("length", "", 3u8, Ranges::u8(0, 100));
    let length = Config::new(
      "length",
      "",
      Duration::from_millis(3),
      Ranges::duration(0, 100),
    );
    let config = ConfigCollection::default();

    config.get(&count);
    config.get(&length);
  }

  #[test]
  fn setting_the_same_value_emits_nothing() {
    let turns = Config::new("turns", "", 3u8, Ranges::u8(1, 5));
    let mut test_ctx = TestContext::default();

    let ctx = test_ctx.svc_ctx();
    ctx.config.set(&turns, 3); // already the default
    ctx.config.set(&turns, 5);
    ctx.config.inc(turns.display()); // already at the max

    let changed = test_ctx
      .events_emitted()
      .iter()
      .filter(|e| e.event.is::<ConfigChanged>())
      .count();
    assert_eq!(changed, 1);
  }

  #[test]
  fn saving_keeps_unused_keys_and_leaves_out_defaults() {
    let turns = Config::new("turns", "", 3u8, Ranges::u8(1, 5));
    let balls = Config::new("balls", "", 3u8, Ranges::u8(1, 5));
    let config = saved("turns = 4\nballs = 2\nunused = 7");

    config.set(&turns, 5);
    config.set(&balls, 3); // back to default

    assert_eq!(
      config.to_table(),
      toml::from_str::<toml::Table>("turns = 5\nunused = 7").unwrap()
    );
  }

  #[test]
  fn reading_alone_saves_nothing_new() {
    let turns = Config::new("turns", "", 3u8, Ranges::u8(1, 5));
    let config = ConfigCollection::default();
    config.get(&turns);
    assert!(config.to_table().is_empty());
  }

  #[tokio::test]
  async fn changes_are_saved_and_reload() {
    let dir = std::env::temp_dir().join(format!("frontbox-config-save-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    std::fs::write(&path, "unused = 7\n").unwrap();

    let turns = Config::new("turns", "", 3u8, Ranges::u8(1, 5));
    let mut config = ConfigCollection::load_from_disk(&path);
    config.begin_save_task(path.clone());
    config.set(&turns, 5);
    config.save();

    let reloaded = tokio::time::timeout(Duration::from_secs(2), async {
      loop {
        let reloaded = ConfigCollection::load_from_disk(&path);
        if reloaded.pending_disk.contains_key("turns") {
          break reloaded;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
      }
    })
    .await
    .expect("the change was never saved");
    assert_eq!(reloaded.get(&turns), 5);
    assert_eq!(
      reloaded.pending_disk.get("unused"),
      Some(&toml::Value::Integer(7))
    );
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
