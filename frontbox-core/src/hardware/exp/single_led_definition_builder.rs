use crate::prelude::*;

pub struct SingleLedDefinitionBuilder {
  name: &'static str,
  tags: Vec<Box<dyn Tag>>,
  channels: LedChannels,
  location: Option<Vec3>,
  config: Option<LedConfiguration>,
}
impl SingleLedDefinitionBuilder {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      tags: Vec::new(),
      channels: LedChannels::Rgb,
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

  /// Uses one channel of its LED address, as one brightness, instead of all three as a color
  pub fn channel(mut self, channel: LedChannel) -> Self {
    self.channels = LedChannels::One(channel);
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

  pub fn channel_order(mut self, order: ChannelOrder) -> Self {
    self.config_mut().channel_order = order;
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
    LedDefinition::new(self.name, self.tags, 1, self.channels, locations, self.config)
  }

  pub(crate) fn name(&self) -> &'static str {
    self.name
  }

  /// Builds the light on `channel`, for a builder that decides channels itself ([`ChannelLedDefinitionBuilder`],
  /// [`PackedLedDefinitionBuilder`]). Panics if the light was already given a channel.
  pub(crate) fn build_on(self, channel: LedChannel) -> SingleLedDefinition {
    if self.channels != LedChannels::Rgb {
      panic!(
        "LED \"{}\" is given a channel, but LedDefinition::channels and LedDefinition::packed decide its channel",
        self.name
      );
    }
    SingleLedDefinition {
      name: LedDefinition::child_name(self.name, 0),
      channels: LedChannels::One(channel),
      tags: self.tags,
      location: self.location,
      config: self.config,
    }
  }

  pub fn q(&self) -> LightQ {
    LightQ::name(self.name)
  }
}
