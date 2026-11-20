use crate::prelude::*;

/// 1 to 3 single-channel lights on one LED address, such as single-channel flashers. Each light takes the next
/// channel, R, G then B, in the order given.
///
/// ```ignore
/// LedDefinition::channels([
///   LedDefinition::single("flasher_left").tag(Flasher),
///   LedDefinition::single("flasher_right").tag(Flasher),
/// ])
/// ```
pub struct ChannelLedDefinitionBuilder {
  lights: Vec<SingleLedDefinitionBuilder>,
}

impl ChannelLedDefinitionBuilder {
  pub fn new(lights: impl IntoIterator<Item = SingleLedDefinitionBuilder>) -> Self {
    Self {
      lights: lights.into_iter().collect(),
    }
  }

  /// Panics with no lights, more than 3, or a light given its own channel
  pub fn build(self) -> LedDefinition {
    let count = self.lights.len();
    if count == 0 || count > CHANNELS.len() {
      panic!("LedDefinition::channels takes 1 to 3 lights, but was given {}", count);
    }
    let children = self
      .lights
      .into_iter()
      .zip(CHANNELS)
      .map(|(light, channel)| light.build_on(channel))
      .collect();
    LedDefinition::from_children(children, vec![0; count])
  }
}

const CHANNELS: [LedChannel; 3] = [LedChannel::R, LedChannel::G, LedChannel::B];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn lights_take_channels_in_order() {
    let def = LedDefinition::channels([
      LedDefinition::single("a"),
      LedDefinition::single("b"),
      LedDefinition::single("c"),
    ])
    .build();
    let channels: Vec<_> = def.children().iter().map(|child| child.channels).collect();
    assert_eq!(
      channels,
      vec![
        LedChannels::One(LedChannel::R),
        LedChannels::One(LedChannel::G),
        LedChannels::One(LedChannel::B)
      ]
    );
    assert_eq!(def.address_count(), 1);
  }

  #[test]
  #[should_panic(expected = "takes 1 to 3 lights")]
  fn more_than_three_lights_panic() {
    LedDefinition::channels((0..4).map(|_| LedDefinition::single("a"))).build();
  }

  #[test]
  #[should_panic(expected = "its place in the list decides its channel")]
  fn a_light_given_its_own_channel_panics() {
    LedDefinition::channels([LedDefinition::single("a").channel(LedChannel::B)]).build();
  }
}
