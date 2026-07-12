use std::time::Duration;

use crate::Tag;
use crate::prelude::{HardwareDefinition, Location};

#[derive(Debug, Clone)]
pub struct SwitchDefinition {
  pub name: &'static str,
  pub tags: Vec<Box<dyn Tag>>,
  pub locations: Vec<Location>,
  pub config: Option<SwitchConfig>,
}

impl SwitchDefinition {
  pub fn new(name: &'static str) -> SwitchDefinitionBuilder {
    SwitchDefinitionBuilder::new(name)
  }
}

impl HardwareDefinition for SwitchDefinition {
  fn name(&self) -> &'static str {
    self.name
  }

  fn tags(&self) -> Vec<Box<dyn Tag>> {
    self.tags.clone()
  }

  fn locations(&self) -> Vec<Location> {
    self.locations.clone()
  }
}

pub struct SwitchDefinitionBuilder {
  name: &'static str,
  tags: Vec<Box<dyn Tag>>,
  location: Vec<Location>,
  config: Option<SwitchConfig>,
}

impl SwitchDefinitionBuilder {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      tags: Vec::new(),
      location: Vec::new(),
      config: None,
    }
  }

  pub fn tag(mut self, tag: impl Tag + 'static) -> Self {
    self.tags.push(Box::new(tag));
    self
  }

  pub fn tags(mut self, tags: impl IntoIterator<Item = Box<dyn Tag>>) -> Self {
    self.tags.extend(tags);
    self
  }

  pub fn location(mut self, location: Location) -> Self {
    self.location.push(location);
    self
  }

  pub fn locations(mut self, locations: impl IntoIterator<Item = Location>) -> Self {
    self.location.extend(locations);
    self
  }

  pub fn config(mut self, config: SwitchConfig) -> Self {
    self.config = Some(config);
    self
  }

  pub fn inverted(mut self) -> Self {
    self.config_mut().inverted = true;
    self
  }

  pub fn debounce_close(mut self, debounce: Duration) -> Self {
    self.config_mut().debounce_close = Some(debounce);
    self
  }

  pub fn debounce_open(mut self, debounce: Duration) -> Self {
    self.config_mut().debounce_open = Some(debounce);
    self
  }

  /// Convenience for setting both open/close debounce to the same value.
  pub fn debounce(mut self, debounce: Duration) -> Self {
    let cfg = self.config_mut();
    cfg.debounce_close = Some(debounce);
    cfg.debounce_open = Some(debounce);
    self
  }

  fn config_mut(&mut self) -> &mut SwitchConfig {
    self.config.get_or_insert_with(SwitchConfig::default)
  }

  pub fn build(self) -> SwitchDefinition {
    SwitchDefinition {
      name: self.name,
      tags: self.tags,
      locations: self.location,
      config: self.config,
    }
  }
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
