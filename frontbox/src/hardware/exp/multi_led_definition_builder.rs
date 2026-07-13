use crate::prelude::*;

pub struct MultiLedDefinitionBuilder {
  name: &'static str,
  tags: Vec<Box<dyn Tag>>,
  count: u16,
  locations: Vec<Vec3>,
  config: Option<LedConfiguration>,
}

impl MultiLedDefinitionBuilder {
  pub fn new(name: &'static str, count: u16) -> Self {
    Self {
      name,
      count,
      tags: Vec::new(),
      locations: Vec::new(),
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

  pub fn location(mut self, location: Vec3) -> Self {
    self.locations.push(location);
    self
  }

  pub fn locations(mut self, locations: impl IntoIterator<Item = Vec3>) -> Self {
    self.locations.extend(locations);
    self
  }

  pub fn config(mut self, config: LedConfiguration) -> Self {
    self.config = Some(config);
    self
  }

  pub fn channels(mut self, channels: LedChannels) -> Self {
    self.config_mut().channels = channels;
    self
  }

  fn config_mut(&mut self) -> &mut LedConfiguration {
    self.config.get_or_insert_with(LedConfiguration::default)
  }

  pub fn build(self) -> MultiLedDefinition {
    let count = if self.count == 0 {
      self.locations.len() as u16
    } else {
      self.count
    };
    MultiLedDefinition::new(self.name, self.tags, count, self.locations, self.config)
  }
}
