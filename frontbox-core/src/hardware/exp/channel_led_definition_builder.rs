use crate::prelude::*;

/// Up to 3 single-channel lights on one LED address, one per channel. A channel given no light is left unused.
///
/// ```ignore
/// LedDefinition::channels()
///   .r(LedDefinition::single("flasher_left").tag(Flasher))
///   .b(LedDefinition::single("flasher_right").tag(Flasher))
/// ```
///
/// This also fits FAST's FP-LUM-0709, a board that takes one LED address and turns its R, G and B into three
/// low-voltage outputs (up to 12V, under 2A for the whole board). Each output used for lighting, such as GI or a
/// strobe, is a light here. To drive an RGB strip from it instead, use one [`LedDefinition::single`].
pub struct ChannelLedDefinitionBuilder {
  lights: [Option<SingleLedDefinitionBuilder>; 3],
}

impl ChannelLedDefinitionBuilder {
  pub fn new() -> Self {
    Self { lights: [None, None, None] }
  }

  pub fn r(self, light: SingleLedDefinitionBuilder) -> Self {
    self.with(LedChannel::R, light)
  }

  pub fn g(self, light: SingleLedDefinitionBuilder) -> Self {
    self.with(LedChannel::G, light)
  }

  pub fn b(self, light: SingleLedDefinitionBuilder) -> Self {
    self.with(LedChannel::B, light)
  }

  /// Panics with no lights, or a light given its own channel
  pub fn build(self) -> LedDefinition {
    let children: Vec<_> = self
      .lights
      .into_iter()
      .zip(LedChannel::ALL)
      .filter_map(|(light, channel)| light.map(|light| light.build_on(channel)))
      .collect();
    if children.is_empty() {
      panic!("LedDefinition::channels was given no lights");
    }
    let offsets = vec![0; children.len()];
    LedDefinition::from_children(children, offsets)
  }

  fn with(mut self, channel: LedChannel, light: SingleLedDefinitionBuilder) -> Self {
    let slot = &mut self.lights[channel as usize];
    if let Some(existing) = slot {
      panic!(
        "LedDefinition::channels was given \"{}\" and \"{}\" for channel {:?}",
        existing.name(),
        light.name(),
        channel
      );
    }
    *slot = Some(light);
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn channels(def: &LedDefinition) -> Vec<LedChannels> {
    def.children().iter().map(|child| child.channels).collect()
  }

  #[test]
  fn each_light_takes_its_channel() {
    let def = LedDefinition::channels()
      .b(LedDefinition::single("c"))
      .r(LedDefinition::single("a"))
      .g(LedDefinition::single("b"))
      .build();
    assert_eq!(
      channels(&def),
      vec![
        LedChannels::One(LedChannel::R),
        LedChannels::One(LedChannel::G),
        LedChannels::One(LedChannel::B)
      ]
    );
    assert_eq!(def.address_count(), 1);
  }

  #[test]
  fn unused_channels_are_skipped() {
    let def = LedDefinition::channels()
      .g(LedDefinition::single("gi"))
      .b(LedDefinition::single("strobe"))
      .build();
    assert_eq!(channels(&def), vec![LedChannels::One(LedChannel::G), LedChannels::One(LedChannel::B)]);
    assert_eq!(def.address_count(), 1);
  }

  #[test]
  #[should_panic(expected = "given no lights")]
  fn no_lights_panic() {
    LedDefinition::channels().build();
  }

  #[test]
  #[should_panic(expected = "given \"a\" and \"b\" for channel G")]
  fn a_channel_given_twice_panics() {
    LedDefinition::channels()
      .g(LedDefinition::single("a"))
      .g(LedDefinition::single("b"));
  }

  #[test]
  #[should_panic(expected = "decide its channel")]
  fn a_light_given_its_own_channel_panics() {
    LedDefinition::channels().r(LedDefinition::single("a").channel(LedChannel::B)).build();
  }
}
