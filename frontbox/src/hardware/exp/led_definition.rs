// TODO: migrate LED to using HardwareDefinition
// TODO: there needs to be an underlying type here that actually produces Vec<HardwareDefinition>
// things that contain multiple LEDs should maybe generate unique names for them, which are later referenceable
// e.g. MultiLedDefinition.child(2).name

use std::borrow::Cow;

use crate::hardware::exp::led_strip_builder::LedStripBuilder;
use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct LedDefinition {
  pub name: Cow<'static, str>,
  pub tags: Vec<Box<dyn Tag>>,
  pub location: Option<Vec3>,
  pub config: Option<LedConfiguration>,
}

impl LedDefinition {
  pub fn single(name: &'static str) -> SingleLedDefinitionBuilder {
    SingleLedDefinitionBuilder::new(name)
  }

  pub fn multi(name: &'static str, count: u16) -> MultiLedDefinitionBuilder {
    MultiLedDefinitionBuilder::new(name, count)
  }

  pub fn strip(name: &'static str, count: u16) -> LedStripBuilder {
    LedStripBuilder::new(name, count)
  }
}

impl HardwareDefinition for LedDefinition {
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

// pub struct LedMatrixDefinition {
//   pub name: &'static str,
//   pub tags: Vec<Box<dyn Tag>>,
//   pub height: u16,
//   pub width: u16,
//   // where CornerLocation is -- { tag: Box<dyn Tag>, rotation: f32, top_left: (f32, f32), bottom_right: (f32, f32) }
//   pub locations: Vec<CornerLocation>,
// }
