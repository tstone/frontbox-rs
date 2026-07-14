// TODO: migrate LED to using HardwareDefinition
// TODO: there needs to be an underlying type here that actually produces Vec<HardwareDefinition>
// things that contain multiple LEDs should maybe generate unique names for them, which are later referenceable
// e.g. MultiLedDefinition.child(2).name

use std::borrow::Cow;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct SingleLedDefinition {
  pub name: Cow<'static, str>,
  pub tags: Vec<Box<dyn Tag>>,
  pub location: Option<Vec3>,
  pub config: Option<LedConfiguration>,
}

impl HardwareDefinition for SingleLedDefinition {
  fn name(&self) -> Cow<'static, str> {
    self.name.clone()
  }

  fn tags(&self) -> Vec<Box<dyn Tag>> {
    self.tags.clone()
  }

  fn location(&self) -> Option<Vec3> {
    self.location
  }
}

#[derive(Debug, Clone, Default)]
pub struct LedConfiguration {
  pub channels: LedChannels,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum LedChannels {
  #[default]
  RGB,
  GRB,
  BRG,
  RGBW,
  GRBW,
  BRGW,
}
