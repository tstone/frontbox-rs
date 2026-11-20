use crate::prelude::*;

/// Single-channel lights laid back to back along the wire, three to an LED address. The first light takes R of the
/// first LED address, the fourth takes R of the second, and so on, so a run of lights can start partway through an
/// LED address. `skip` leaves wire positions unused.
///
/// ```ignore
/// LedDefinition::packed()
///   .light(LedDefinition::single("segment_a"))
///   .light(LedDefinition::single("segment_b"))
///   .skip(1)
///   .light(LedDefinition::single("segment_c"))
/// ```
///
/// For hardware that packs one-color outputs onto addressable LED drivers, such as segment displays.
pub struct PackedLedDefinitionBuilder {
  lights: Vec<(u16, SingleLedDefinitionBuilder)>,
  next_position: u16,
}

impl PackedLedDefinitionBuilder {
  pub fn new() -> Self {
    Self { lights: Vec::new(), next_position: 0 }
  }

  /// Puts the light on the next wire position
  pub fn light(mut self, light: SingleLedDefinitionBuilder) -> Self {
    self.lights.push((self.next_position, light));
    self.next_position += 1;
    self
  }

  pub fn lights(self, lights: impl IntoIterator<Item = SingleLedDefinitionBuilder>) -> Self {
    lights.into_iter().fold(self, Self::light)
  }

  /// Leaves the next `count` wire positions unused
  pub fn skip(mut self, count: u16) -> Self {
    self.next_position += count;
    self
  }

  /// Panics with no lights, or a light given its own channel
  pub fn build(self) -> LedDefinition {
    if self.lights.is_empty() {
      panic!("LedDefinition::packed was given no lights");
    }
    let (offsets, children) = self
      .lights
      .into_iter()
      .map(|(position, light)| (position / 3, light.build_on(LedChannel::ALL[(position % 3) as usize])))
      .unzip();
    LedDefinition::from_children(children, offsets)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn placements(def: &LedDefinition) -> Vec<(u16, LedChannels)> {
    (0..def.children().len() as u16)
      .map(|i| (def.offset(i).unwrap(), def.child(i).unwrap().channels))
      .collect()
  }

  fn lights(names: &[&'static str]) -> Vec<SingleLedDefinitionBuilder> {
    names.iter().map(|name| LedDefinition::single(name)).collect()
  }

  #[test]
  fn lights_fill_each_led_address_in_order() {
    let def = LedDefinition::packed().lights(lights(&["a", "b", "c", "d"])).build();
    assert_eq!(
      placements(&def),
      vec![
        (0, LedChannels::One(LedChannel::R)),
        (0, LedChannels::One(LedChannel::G)),
        (0, LedChannels::One(LedChannel::B)),
        (1, LedChannels::One(LedChannel::R)),
      ]
    );
    assert_eq!(def.address_count(), 2);
  }

  #[test]
  fn skipped_positions_are_left_unused() {
    let def = LedDefinition::packed()
      .light(LedDefinition::single("a"))
      .skip(3)
      .light(LedDefinition::single("b"))
      .build();
    assert_eq!(
      placements(&def),
      vec![(0, LedChannels::One(LedChannel::R)), (1, LedChannels::One(LedChannel::G))]
    );
  }

  #[test]
  fn keeps_each_lights_name() {
    let def = LedDefinition::packed().lights(lights(&["a", "b"])).build();
    assert!(def.names().contains(LedDefinition::child_name("b", 0).as_ref()));
  }

  #[test]
  #[should_panic(expected = "given no lights")]
  fn no_lights_panic() {
    LedDefinition::packed().skip(2).build();
  }

  #[test]
  #[should_panic(expected = "decide its channel")]
  fn a_light_given_its_own_channel_panics() {
    LedDefinition::packed().light(LedDefinition::single("a").channel(LedChannel::B)).build();
  }
}
