use crate::prelude::*;

pub struct MultiLedDefinitionBuilder {
  name: &'static str,
  tags: Vec<Box<dyn Tag>>,
  count: Option<u16>,
  locations: Vec<Vec3>,
  config: Option<LedConfiguration>,
}

impl MultiLedDefinitionBuilder {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      count: None,
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

  /// Number of LEDs, only needed when no locations are given -- otherwise there is one LED per location
  pub fn count(mut self, count: u16) -> Self {
    self.count = Some(count);
    self
  }

  /// One LED per location, e.g. from [`LedLayout`]
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

  pub fn build(self) -> LedDefinition {
    let located = self.locations.len() as u16;
    let count = match self.count {
      Some(count) if located > 0 && count != located => {
        log::warn!(
          "LED definition \"{}\" has a count of {} but {} locations",
          self.name,
          count,
          located
        );
        count
      }
      Some(count) => count,
      None => located,
    };
    LedDefinition::new(self.name, self.tags, count, self.locations, self.config)
  }
}
