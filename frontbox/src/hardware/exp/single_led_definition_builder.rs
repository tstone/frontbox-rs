use crate::prelude::*;

pub struct SingleLedDefinitionBuilder {
  name: &'static str,
  tags: Vec<Box<dyn Tag>>,
  location: Option<Vec3>,
  config: Option<LedConfiguration>,
}
impl SingleLedDefinitionBuilder {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      tags: Vec::new(),
      location: None,
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
    self.location = Some(location);
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
    let locations = match self.location {
      Some(loc) => vec![loc],
      None => Vec::new(),
    };
    LedDefinition::new(self.name, self.tags, 1, locations, self.config)
  }

  pub fn q(&self) -> LedQ {
    LedQ::name(self.name)
  }
}
