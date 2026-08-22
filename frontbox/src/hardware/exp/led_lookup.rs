use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct LedLookup {
  by_name: HashMap<String, LED>,
  configs: HashMap<String, LedConfiguration>,
  channel_by_address: HashMap<LedAddress, LedChannels>,
}

impl LedLookup {
  pub fn new(boards: &Vec<ResolvedExpansionBoard>) -> Self {
    let mut by_name = HashMap::new();
    let mut configs = HashMap::new();
    let mut channel_by_address = HashMap::new();

    for board in boards {
      for port in &board.led_ports {
        for led in &port.leds {
          let address = LedAddress {
            exp: led.assignment.clone(),
            index: led.id as u16,
          };

          let channel = if let Some(config) = &led.definition.config {
            config.channels
          } else {
            LedChannels::default()
          };
          channel_by_address.insert(address.clone(), channel);

          by_name.insert(
            led.definition.name.to_string(),
            LED {
              name: led.definition.name.to_string(),
              address,
              tags: led.definition.tags.clone(),
              location: led.definition.location,
            },
          );

          led
            .definition
            .config
            .as_ref()
            .and_then(|config| configs.insert(led.definition.name.to_string(), config.clone()));
        }
      }
    }

    Self {
      by_name,
      configs,
      channel_by_address,
    }
  }

  pub fn by_name(&self, name: &str) -> Option<&LED> {
    self.by_name.get(name)
  }

  pub fn by_tag<T: Tag + 'static>(&self) -> Vec<&LED> {
    self
      .by_name
      .values()
      .filter(|led| {
        led
          .tags
          .iter()
          .any(|tag| <dyn Tag>::as_any(&**tag).is::<T>())
      })
      .collect()
  }

  pub fn query(&self, selection: &HardwareQuery) -> Vec<&LED> {
    self
      .by_name
      .values()
      .filter(|illum| selection.matches_led(illum))
      .collect()
  }

  pub fn config(&self, name: &'static str) -> Option<&LedConfiguration> {
    self
      .by_name
      .get(name)
      .and_then(|led| self.configs.get(&led.name))
  }

  pub(crate) fn color_channels_by_id(&self, addr: &LedAddress) -> LedChannels {
    match self.channel_by_address.get(&addr) {
      Some(channels) => *channels,
      None => LedChannels::default(),
    }
  }
}

impl Deref for LedLookup {
  type Target = HashMap<String, LED>;

  fn deref(&self) -> &Self::Target {
    &self.by_name
  }
}

impl DerefMut for LedLookup {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.by_name
  }
}
