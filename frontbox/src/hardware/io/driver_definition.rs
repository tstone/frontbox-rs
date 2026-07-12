use crate::prelude::*;
use crate::{DriverMode, Tag};

#[derive(Debug, Clone)]
pub struct DriverDefinition {
  pub name: &'static str,
  pub tags: Vec<Box<dyn Tag>>,
  pub locations: Vec<Location>,
  pub mode: Option<Box<dyn DriverMode>>,
}

impl DriverDefinition {
  pub fn new(name: &'static str) -> DriverDefinitionBuilder {
    DriverDefinitionBuilder::new(name)
  }

  /// Lamps use low-voltage drivers to turn on and off
  pub fn lamp(name: &'static str) -> LampDefinitionBuilder {
    LampDefinitionBuilder::new(name)
  }
}

impl HardwareDefinition for DriverDefinition {
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

pub struct DriverDefinitionBuilder {
  name: &'static str,
  tags: Vec<Box<dyn Tag>>,
  locations: Vec<Location>,
  mode: Option<Box<dyn DriverMode>>,
}

impl DriverDefinitionBuilder {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      tags: Vec::new(),
      locations: Vec::new(),
      mode: None,
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
    self.locations.push(location);
    self
  }

  pub fn locations(mut self, locations: impl IntoIterator<Item = Location>) -> Self {
    self.locations.extend(locations);
    self
  }

  pub fn mode(mut self, mode: impl DriverMode + 'static) -> Self {
    self.mode = Some(Box::new(mode));
    self
  }

  pub fn build(self) -> DriverDefinition {
    DriverDefinition {
      name: self.name,
      tags: self.tags,
      locations: self.locations,
      mode: self.mode,
    }
  }
}

pub struct LampDefinitionBuilder {
  name: &'static str,
  tags: Vec<Box<dyn Tag>>,
  locations: Vec<Location>,
}

impl LampDefinitionBuilder {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      tags: vec![Box::new(tags::_FrontboxDrivenLamp)],
      locations: Vec::new(),
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
    self.locations.push(location);
    self
  }

  pub fn locations(mut self, locations: impl IntoIterator<Item = Location>) -> Self {
    self.locations.extend(locations);
    self
  }

  pub fn build(self) -> DriverDefinition {
    DriverDefinition {
      name: self.name,
      tags: self.tags,
      locations: self.locations,
      mode: Some(Box::new(PulseHoldMode {
        trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
        initial_pwm_power: Power::ZERO,
        secondary_pwm_power: Power::FULL,
        ..Default::default()
      })),
    }
  }
}
