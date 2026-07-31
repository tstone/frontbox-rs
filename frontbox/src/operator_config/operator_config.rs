use std::any::Any;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use tokio::sync::mpsc;

use crate::operator_config::{ConfigValue, Domain, GeneralizedConfigValue};
use crate::prelude::System;
use crate::prelude::app_message::AppMessage;

pub struct OperatorConfig {
  current_values: HashMap<&'static str, Box<dyn Any + Send + Sync>>,
  pending_disk: HashMap<String, toml::Value>, // raw until a matching ConfigValue registers
  pub(crate) app_sender: Option<mpsc::UnboundedSender<AppMessage>>,
}

impl OperatorConfig {
  pub fn new() -> Self {
    Self {
      current_values: HashMap::new(),
      pending_disk: HashMap::new(),
      app_sender: None,
    }
  }

  /// Reads values from disk into a temporary buffer, but waits until a config value is registered before assignment
  pub fn load_from_disk(path: &Path) -> Self {
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
  pub fn register(&mut self, cv: &'static dyn GeneralizedConfigValue) {
    if self.current_values.contains_key(cv.text()) {
      return; // already registered — safe if the same ConfigValue is reachable from two defs
    }
    match self.pending_disk.get(cv.text()) {
      Some(raw) => cv.load_from_toml(raw, &mut self.current_values),
      None => cv.insert_default(&mut self.current_values),
    }
  }

  pub fn get<T, D>(&self, config: &ConfigValue<T, D>) -> T
  where
    T: Clone + Send + Sync + 'static,
    D: Domain<T>,
  {
    match self.current_values.get(config.name) {
      Some(v) => v
        .downcast_ref::<T>()
        .expect("config type mismatch for key")
        .clone(),
      None => config.default.clone(),
    }
  }

  pub fn set<T, D>(&mut self, config: &ConfigValue<T, D>, value: T)
  where
    T: Clone + Send + Sync + 'static,
    D: Domain<T>,
  {
    self.current_values.insert(config.name, Box::new(value));

    if let Some(app_sender) = &self.app_sender {
      let event = OperatorConfigChanged(config.name);
      let type_id = event.type_id();
      let _ = app_sender.send(AppMessage::EmitEvent(Box::new(event), type_id));
    }
  }
}

impl System for OperatorConfig {}

pub struct OperatorConfigChanged(pub &'static str);
