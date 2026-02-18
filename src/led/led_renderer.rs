use std::collections::{HashMap, HashSet};

use crate::machine::serial_interface::SerialInterface;
use crate::prelude::*;
use crate::protocol::prelude::SetLedCommand;

const LED_SET_BATCH_SIZE: usize = 24;

pub struct LedRenderer {
  led_map: HashMap<&'static str, AddressableLed>,
  set_leds: HashSet<&'static str>,
}

impl LedRenderer {
  pub fn new(expansion_boards: &Vec<ExpansionBoardSpec>) -> Self {
    let mut led_map = HashMap::new();
    for board in expansion_boards {
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
            },
          );
        }
      }
    }

    Self {
      led_map,
      set_leds: HashSet::new(),
    }
  }

  pub async fn render(
    &mut self,
    exp_port: &mut SerialInterface,
    led_declarations: Vec<LedDeclaration>,
  ) {
    // apply incomming colors
    let updated_led_names = self.set_bulk(exp_port, led_declarations).await;

    // clear out previously set LEDS that are not on this frame
    let declarations = self
      .set_leds
      .iter()
      // don't reset LEDs updated in this frame
      .filter(|name| !updated_led_names.contains(**name))
      .map(|n| LedDeclaration::new(n, LedState::Off))
      .collect::<Vec<_>>();
    self.set_bulk(exp_port, declarations).await;

    self.set_leds = updated_led_names;
  }

  async fn set_bulk(
    &mut self,
    exp_port: &mut SerialInterface,
    led_declarations: Vec<LedDeclaration>,
  ) -> HashSet<&'static str> {
    let mut updated_led_names = HashSet::new();
    let mut leds_to_set: HashMap<LedAddress, Vec<(u16, Color)>> = HashMap::new();
    let off_color = Color::black();

    for decl in led_declarations {
      if let Some(led) = self.led_map.get(decl.name) {
        let color = match decl.state {
          LedState::On(c) => c,
          LedState::Off => off_color.clone(),
        };

        if color != off_color {
          updated_led_names.insert(decl.name);
        }

        match leds_to_set.get_mut(&led.address) {
          Some(list) => {
            list.push((led.index, color));
          }
          None => {
            leds_to_set.insert(led.address.clone(), vec![(led.index, color)]);
          }
        }
      }
    }

    // set LEDs by board/port in batches
    for address in leds_to_set.keys() {
      for chunk in leds_to_set[&address].chunks(LED_SET_BATCH_SIZE) {
        let cmd = SetLedCommand::new(address.address, address.breakout, chunk.to_vec());
        let _ = exp_port.dispatch(&cmd).await;
      }
    }

    updated_led_names
  }
}

#[derive(Debug, Clone)]
struct AddressableLed {
  pub address: LedAddress,
  pub index: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LedAddress {
  pub address: u8,
  pub breakout: Option<u8>,
  pub port: u8,
}
