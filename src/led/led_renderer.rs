use std::collections::{HashMap, HashSet};

use crate::machine::serial_interface::SerialInterface;
use crate::prelude::*;
use crate::protocol::prelude::SetLedCommand;

const LED_SET_BATCH_SIZE: usize = 24;

pub struct LedRenderer {
  led_map: HashMap<&'static str, AddressableLed>,
  set_leds: HashSet<&'static str>,
  resolver: Box<dyn LedResolver>,
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
      // resolver: Box::new(BezierMixResolver::new()),
      resolver: Box::new(AlternateResolver::new()),
    }
  }

  pub fn reset(&mut self) {
    // TODO: LED renderer needs to remember current state and not require a reset
    self.set_leds.clear();
    self.resolver.reset();
  }

  pub fn tick(&mut self, delta: Duration) {
    self.resolver.tick(delta);
  }

  pub async fn render(
    &mut self,
    exp_port: &mut SerialInterface,
    led_declarations: HashMap<u64, HashMap<&'static str, LedState>>,
  ) {
    let black = Color::black();

    // group declarations by LED name, finding conflicts
    let mut conflicts: HashMap<&'static str, Vec<(u64, LedState)>> = HashMap::new();
    let mut led_temp_updates: HashMap<&'static str, (u64, LedState)> = HashMap::new();

    for (system_id, states) in led_declarations {
      for (led_name, state) in states {
        if let Some(conflict_list) = conflicts.get_mut(led_name) {
          conflict_list.push((system_id, state));
        } else if led_temp_updates.contains_key(led_name) {
          let current = led_temp_updates.remove(led_name).unwrap();
          conflicts.insert(led_name, vec![current, (system_id, state)]);
        } else {
          led_temp_updates.insert(led_name, (system_id, state));
        }
      }
    }

    // resolve conflicts
    for (led_name, conflict_list) in conflicts {
      let resolved = self.resolver.resolve(led_name, conflict_list);
      led_temp_updates.insert(led_name, (0, resolved));
    }

    // flatten to just name/state
    let mut leds_set_this_frame = HashSet::new();
    let mut led_updates: HashMap<&'static str, LedState> = HashMap::new();
    for (led_name, (_, state)) in led_temp_updates {
      match &state {
        LedState::On(c) if c != &black => {
          leds_set_this_frame.insert(led_name);
        }
        _ => {}
      }

      led_updates.insert(led_name, state);
    }

    // clear out previously set LEDS that are not on this frame
    for name in self.set_leds.difference(&leds_set_this_frame) {
      led_updates.insert(name, LedState::Off);
    }

    self.set_bulk(exp_port, led_updates).await;
    self.set_leds = leds_set_this_frame;
  }

  async fn set_bulk(
    &mut self,
    exp_port: &mut SerialInterface,
    led_declarations: HashMap<&'static str, LedState>,
  ) -> HashSet<&'static str> {
    let mut updated_led_names = HashSet::new();
    let mut leds_to_set: HashMap<LedAddress, Vec<(u16, Color)>> = HashMap::new();
    let off_color = Color::black();

    for (led_name, state) in led_declarations {
      if let Some(led) = self.led_map.get(led_name) {
        let color = match state {
          LedState::On(c) => c,
          LedState::Off => off_color.clone(),
        };

        if color != off_color {
          updated_led_names.insert(led_name);
        }

        match leds_to_set.get_mut(&led.address) {
          Some(list) => {
            list.push((led.index, color));
          }
          None => {
            leds_to_set.insert(led.address.clone(), vec![(led.index, color)]);
          }
        }
      } else {
        log::warn!("Received LED declaration for unknown LED '{}'", led_name);
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
