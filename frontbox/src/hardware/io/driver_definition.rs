use std::borrow::Cow;

use crate::operator_config::HardwareValue;
use crate::prelude::*;
use crate::{DriverMode, Tag};

#[derive(Debug, Clone)]
pub struct DriverDefinition {
  pub name: &'static str,
  pub tags: Vec<Box<dyn Tag>>,
  pub location: Option<Vec3>,
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
  fn name(&self) -> Cow<'static, str> {
    Cow::Borrowed(self.name)
  }

  fn tags(&self) -> Vec<Box<dyn Tag>> {
    self.tags.clone()
  }

  fn location(&self) -> Option<Vec3> {
    self.location
  }
}

pub struct DriverDefinitionBuilder {
  name: &'static str,
  tags: Vec<Box<dyn Tag>>,
  location: Option<Vec3>,
  mode: Option<Box<dyn DriverMode>>,
}

impl DriverDefinitionBuilder {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      tags: Vec::new(),
      location: None,
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

  pub fn location(mut self, location: Vec3) -> Self {
    self.location = Some(location);
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
      location: self.location,
      mode: self.mode,
    }
  }
}

pub struct LampDefinitionBuilder {
  name: &'static str,
  tags: Vec<Box<dyn Tag>>,
  location: Option<Vec3>,
}

impl LampDefinitionBuilder {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      tags: vec![Box::new(tags::_FrontboxDrivenLamp)],
      location: None,
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

  pub fn location(mut self, location: Vec3) -> Self {
    self.location = Some(location);
    self
  }

  pub fn build(self) -> DriverDefinition {
    DriverDefinition {
      name: self.name,
      tags: self.tags,
      location: self.location,
      mode: Some(Box::new(PulseHoldMode {
        trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
        initial_pwm_power: HardwareValue::Fixed(Power::ZERO),
        secondary_pwm_power: HardwareValue::Fixed(Power::FULL),
        ..Default::default()
      })),
    }
  }
}
