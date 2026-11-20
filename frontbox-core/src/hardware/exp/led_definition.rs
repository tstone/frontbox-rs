use indexmap::IndexSet;
use std::borrow::Cow;

use crate::prelude::*;

#[derive(Debug)]
pub struct LedDefinition {
  names: IndexSet<String>,
  children: Vec<SingleLedDefinition>,
  /// Each child's LED address, counted from the definition's first
  offsets: Vec<u16>,
}

impl LedDefinition {
  pub fn new(
    name: &'static str,
    tags: Vec<Box<dyn Tag>>,
    count: u16,
    channels: LedChannels,
    locations: Vec<Vec3>,
    config: Option<LedConfiguration>,
  ) -> Self {
    let children: Vec<SingleLedDefinition> = (0..count)
      .map(|index| SingleLedDefinition {
        name: Self::child_name(name, index),
        channels,
        tags: tags.clone(),
        location: locations.get(index as usize).map(|loc| *loc),
        config: config.clone(),
      })
      .collect();
    Self::from_children(children, (0..count).collect())
  }

  pub(crate) fn from_children(children: Vec<SingleLedDefinition>, offsets: Vec<u16>) -> Self {
    Self {
      names: children
        .iter()
        .map(|child| child.name.to_string())
        .collect::<IndexSet<_>>(),
      children,
      offsets,
    }
  }

  pub fn single(name: &'static str) -> SingleLedDefinitionBuilder {
    SingleLedDefinitionBuilder::new(name)
  }

  pub fn multi(name: &'static str) -> MultiLedDefinitionBuilder {
    MultiLedDefinitionBuilder::new(name)
  }

  /// Up to 3 single-channel lights on one LED address, one per channel, such as single-channel flashers or the
  /// outputs of a FAST FP-LUM-0709. See [`ChannelLedDefinitionBuilder`].
  pub fn channels() -> ChannelLedDefinitionBuilder {
    ChannelLedDefinitionBuilder::new()
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

  /// The child's LED address, counted from the definition's first
  pub fn offset(&self, index: u16) -> Option<u16> {
    self.offsets.get(index as usize).copied()
  }

  /// How many LED addresses the definition takes on its port
  pub fn address_count(&self) -> u16 {
    self.offsets.iter().max().map_or(0, |last| last + 1)
  }

  pub fn names(&self) -> &IndexSet<String> {
    &self.names
  }

  /// Query for LED(s) in this definition
  pub fn q(&self) -> LightQ {
    LightQ::names(&self.names)
  }
}

impl Into<LightQ> for LedDefinition {
  fn into(self) -> LightQ {
    self.q()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_children() {
    let def = LedDefinition::multi("test").count(6);
    assert_eq!(def.build().children().len(), 6);
  }

  #[test]
  fn test_names() {
    let def = LedDefinition::multi("test").count(8);
    assert_eq!(def.build().names().len(), 8);
  }

  #[test]
  fn test_channels() {
    assert_eq!(LedDefinition::single("rgb").build().children()[0].channels, LedChannels::Rgb);
    let def = LedDefinition::single("flasher").channel(LedChannel::G).build();
    assert_eq!(def.children()[0].channels, LedChannels::One(LedChannel::G));
  }

  #[test]
  fn test_address_count() {
    assert_eq!(LedDefinition::multi("strip").count(5).build().address_count(), 5);
    let def = LedDefinition::channels()
      .r(LedDefinition::single("a"))
      .g(LedDefinition::single("b"))
      .build();
    assert_eq!(def.address_count(), 1);
  }
}
