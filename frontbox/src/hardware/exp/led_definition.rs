use indexmap::IndexSet;
use std::borrow::Cow;

use crate::hardware::exp::led_strip_builder::LedStripBuilder;
use crate::prelude::*;

#[derive(Debug)]
pub struct LedDefinition {
  names: IndexSet<String>,
  children: Vec<SingleLedDefinition>,
}

impl LedDefinition {
  pub fn new(
    name: &'static str,
    tags: Vec<Box<dyn Tag>>,
    count: u16,
    locations: Vec<Vec3>,
    config: Option<LedConfiguration>,
  ) -> Self {
    let children: Vec<SingleLedDefinition> = (0..count)
      .map(|index| SingleLedDefinition {
        name: Self::child_name(name, index),
        tags: tags.clone(),
        location: locations.get(index as usize).map(|loc| *loc),
        config: config.clone(),
      })
      .collect();
    Self {
      names: children
        .iter()
        .map(|child| child.name.to_string())
        .collect::<IndexSet<_>>(),
      children,
    }
  }

  pub fn single(name: &'static str) -> SingleLedDefinitionBuilder {
    SingleLedDefinitionBuilder::new(name)
  }

  pub fn multi(name: &'static str, count: u16) -> MultiLedDefinitionBuilder {
    MultiLedDefinitionBuilder::new(name, count)
  }

  pub fn strip(name: &'static str, count: u16) -> LedStripBuilder {
    LedStripBuilder::new(name, count)
  }

  pub(crate) fn child_name(name: &'static str, index: u16) -> Cow<'static, str> {
    Cow::Owned(format!("___child::{}::{}", name, index))
  }

  pub fn child(&self, index: u16) -> Option<&SingleLedDefinition> {
    self.children.get(index as usize)
  }

  pub fn children(&self) -> &Vec<SingleLedDefinition> {
    &self.children
  }

  pub fn names(&self) -> &IndexSet<String> {
    &self.names
  }

  /// Query for LED(s) in this definition
  pub fn q(&self) -> HardwareQuery {
    Q::names(&self.names)
  }
}

impl Into<HardwareQuery> for LedDefinition {
  fn into(self) -> HardwareQuery {
    self.q()
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_children() {
    let def = LedDefinition::strip("test", 6);
    assert_eq!(def.build().children().len(), 6);
  }

  #[test]
  fn test_names() {
    let def = LedDefinition::strip("test", 8);
    assert_eq!(def.build().names().len(), 8);
  }
}