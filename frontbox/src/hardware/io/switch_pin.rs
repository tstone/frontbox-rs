use std::time::Duration;

use crate::{HardwareTag, NativeIdentity};

pub struct SwitchPin(pub u16);

impl SwitchPin {
  pub fn named(&self, name: &'static str) -> SwitchMapping {
    SwitchMapping {
      key: name,
      pin: self.0,
      tags: Vec::new(),
      config: None,
    }
  }
}

/// Define a switch pin on an IO board
pub fn switch(pin: u16) -> SwitchPin {
  SwitchPin(pin)
}

pub struct SwitchMapping {
  pub(crate) key: &'static str,
  pub(crate) pin: u16,
  pub(crate) tags: Vec<Box<dyn HardwareTag>>,
  pub(crate) config: Option<SwitchConfig>,
}

impl SwitchMapping {
  pub fn tagged(mut self, tag: impl HardwareTag + 'static) -> Self {
    self.tags.push(Box::new(tag));
    self
  }

  pub fn config(mut self, config: SwitchConfig) -> Self {
    self.config = Some(config);
    self
  }
}

#[derive(Clone)]
pub struct SwitchDefinition {
  pub id: usize,
  pub name: &'static str,
  pub native: NativeIdentity,
  pub config: Option<SwitchConfig>,
  pub tags: Vec<Box<dyn HardwareTag>>,
}

#[derive(Clone, Debug)]
pub struct SwitchConfig {
  pub inverted: bool,
  pub debounce_close: Option<Duration>,
  pub debounce_open: Option<Duration>,
}

impl Default for SwitchConfig {
  fn default() -> Self {
    Self {
      inverted: false,
      debounce_close: None,
      debounce_open: None,
    }
  }
}
