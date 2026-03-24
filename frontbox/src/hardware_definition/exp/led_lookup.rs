use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::prelude::*;
use fast_protocol::Color;
use serde::Serialize;

use crate::ExpansionBoard;

/// A lookup for the machine's LEDs, mapping from LED name to its address and current color
#[derive(Debug, Serialize, Storable)]
pub struct LedLookup {
  led_map: HashMap<&'static str, AddressableLed>,
}

impl LedLookup {
  pub fn new(expansion_boards: &Vec<ExpansionBoard>) -> Self {
    let mut led_map = HashMap::new();

    for board in expansion_boards.iter() {
      for led_port in &board.led_ports {
        for (i, name) in led_port.leds.iter().enumerate() {
          led_map.insert(
            *name,
            AddressableLed {
              address: LedAddress {
                address: board.address,
                breakout: board.breakout,
                port: led_port.port,
              },
              index: i as u16,
              color: Color::black(),
            },
          );
        }
      }
    }

    Self { led_map }
  }

  pub fn get_color(&self, name: &str) -> Option<Color> {
    self.led_map.get(name).map(|led| led.color)
  }

  pub fn set_color(&mut self, name: &str, color: Color) {
    if let Some(led) = self.led_map.get_mut(name) {
      led.color = color;
    }
  }
}

impl Deref for LedLookup {
  type Target = HashMap<&'static str, AddressableLed>;

  fn deref(&self) -> &Self::Target {
    &self.led_map
  }
}

impl DerefMut for LedLookup {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.led_map
  }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddressableLed {
  pub address: LedAddress,
  pub index: u16,
  pub color: Color,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct LedAddress {
  pub address: u8,
  pub breakout: Option<u8>,
  pub port: u8,
}
