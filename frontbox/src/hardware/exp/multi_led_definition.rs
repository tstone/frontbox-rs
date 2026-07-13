use indexmap::IndexSet;
use std::borrow::Cow;

use crate::prelude::*;

#[derive(Debug)]
pub struct MultiLedDefinition {
  names: IndexSet<String>,
  children: Vec<LedDefinition>,
}

impl MultiLedDefinition {
  pub fn new(
    name: &'static str,
    tags: Vec<Box<dyn Tag>>,
    count: u16,
    locations: Vec<Vec3>,
    config: Option<LedConfiguration>,
  ) -> Self {
    let children: Vec<LedDefinition> = (0..count)
      .map(|index| LedDefinition {
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

  pub(crate) fn child_name(name: &'static str, index: u16) -> Cow<'static, str> {
    Cow::Owned(format!("___child::{}::{}", name, index))
  }

  pub fn child(&self, index: u16) -> Option<&LedDefinition> {
    self.children.get(index as usize)
  }

  pub fn children(&self) -> &Vec<LedDefinition> {
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
