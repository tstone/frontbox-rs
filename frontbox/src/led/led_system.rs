use std::collections::HashMap;
use std::i8;

use crate::prelude::*;

const LED_SET_BATCH_SIZE: usize = 24;
pub struct LedSystem {
  all_leds: Vec<AddressableLed>,
  // Rule: Systems cannot contradict themselves. Declarations are thus unique by led/system/z-index.
  declarations: HashMap<AddressableLed, HashMap<DeclarationIdentifier, StatefulLedDeclaration>>,
  prior_render: HashMap<AddressableLed, Color>,
  conflict_resolution: HashMap<AddressableLed, LedConflictResolution>,
  alternate_resolver: AlternateResolver,
}

impl LedSystem {
  pub fn new() -> Self {
    Self {
      declarations: HashMap::new(),
      all_leds: Vec::new(),
      prior_render: HashMap::new(),
      conflict_resolution: HashMap::new(),
      alternate_resolver: AlternateResolver::new(),
    }
  }

  pub fn declare(&mut self, owning_system: u64, declarations: impl Into<LedDeclarations>) {
    let declarations: LedDeclarations = declarations.into();
    for (led, color) in declarations.pairings {
      // A color value of None is the same as undeclaring
      if color.is_none() {
        self.undeclare(owning_system, (led, Some(declarations.z_index)));
        continue;
      }

      let declaration = StatefulLedDeclaration {
        owning_system,
        active: true,
        color: color.unwrap(),
        z_index: declarations.z_index,
      };
      self
        .declarations
        .entry(led.clone())
        .or_insert_with(HashMap::new)
        .insert(
          DeclarationIdentifier {
            system_id: owning_system,
            z_index: declarations.z_index,
          },
          declaration,
        );
    }
  }

  /// Remove declarations for the LED. If z-index is provided, only remove declarations for that layer.
  pub fn undeclare(&mut self, system_id: u64, identifications: impl Into<LedIdentifications>) {
    let identifications: LedIdentifications = identifications.into();
    for led in identifications.leds {
      if let Some(declarations) = self.declarations.get_mut(&led) {
        declarations
          .retain(|d, _| d.system_id != system_id && d.z_index != identifications.z_index);
        if declarations.is_empty() {
          self.declarations.remove(&led);
        }
      }
    }
  }

  /// Keep declarations but mark them as inactive so they don't render
  pub fn deactivate_by_system(&mut self, system_id: u64) {
    for declarations in self.declarations.values_mut() {
      for declaration in declarations.values_mut() {
        if declaration.owning_system == system_id {
          declaration.active = false;
        }
      }
    }
  }

  /// Mark any existing declarations as active
  pub fn activate_by_system(&mut self, system_id: u64) {
    for declarations in self.declarations.values_mut() {
      for declaration in declarations.values_mut() {
        if declaration.owning_system == system_id {
          declaration.active = true;
        }
      }
    }
  }

  pub fn set_conflict_resolution(
    &mut self,
    led: &AddressableLed,
    resolution: LedConflictResolution,
  ) {
    self.conflict_resolution.insert(led.clone(), resolution);
  }
}

impl System for LedSystem {
  fn on_startup(&mut self, ctx: &Context, _systems: &Systems) {
    // Create a copy of all LEDs to reference during rendering
    for board in &ctx.exp_network {
      for port in &board.led_ports {
        for illuminations in &port.illuminations {
          for led in &illuminations.leds {
            self.all_leds.push(led.clone());
          }
        }
      }
    }
  }

  fn on_tick(&mut self, delta: Duration, _ctx: &Context, _systems: &Systems) {
    self.alternate_resolver.accumulate(delta);
  }

  fn on_render(&mut self, _ctx: &Context, systems: &Systems) {
    let mut leds_to_set: Vec<(AddressableLed, Color)> = Vec::new();
    for led in self.all_leds.iter() {
      if let Some(declarations) = self.declarations.get(led) {
        // take only active, highest z-index declaration for each LED
        let active = declarations.values().filter(|d| d.active);
        let maz_z_index = active.clone().map(|d| d.z_index).max().unwrap_or(i8::MIN);
        let top_declarations = active
          .filter(|d| d.z_index == maz_z_index)
          .collect::<Vec<_>>();

        if top_declarations.len() == 1 {
          leds_to_set.push((led.clone(), top_declarations[0].color));
        } else if top_declarations.len() > 1 {
          let resolution_strategy = self
            .conflict_resolution
            .get(led)
            .unwrap_or(&LedConflictResolution::FirstWins);

          match resolution_strategy {
            LedConflictResolution::FirstWins => {
              leds_to_set.push((led.clone(), top_declarations[0].color));
            }
            LedConflictResolution::Mix => {
              leds_to_set.push((
                led.clone(),
                top_declarations
                  .iter()
                  .map(|d| d.color)
                  .collect::<Vec<_>>()
                  .mix_all(),
              ));
            }
            LedConflictResolution::Alternate => {
              let colors = top_declarations.iter().map(|d| d.color).collect();
              leds_to_set.push((
                led.clone(),
                self.alternate_resolver.resolve(led.clone(), colors),
              ));
            }
          }
        }
      } else {
        // no declarations for this LED = turn it off
        // if it's already off this will get filtered out below
        leds_to_set.push((led.clone(), Color::default()));
      }
    }

    // filter out LEDs that are the same as the prior render to avoid redundant updates
    // "there are hard limits to the amount of data that can go out over the EXP bus of about 80 characters per millisecond" -- ecurtz
    leds_to_set.retain(|(led, color)| {
      if let Some(prior_color) = self.prior_render.get(led) {
        if prior_color == color {
          return false;
        }
      }
      self.prior_render.insert(led.clone(), *color);
      true
    });

    if leds_to_set.len() > 0 {
      // group by address to send to machine
      let outgoing: HashMap<LedAddress, Vec<(u16, Color)>> =
        leds_to_set
          .into_iter()
          .fold(HashMap::new(), |mut acc, (led, color)| {
            acc
              .entry(led.address.clone())
              .or_insert_with(Vec::new)
              .push((led.index, color));
            acc
          });

      let machine = systems.expect::<Machine>();
      for (address, leds) in outgoing.into_iter() {
        for chunk in leds.chunks(LED_SET_BATCH_SIZE) {
          machine.set_leds(address.address, address.breakout, chunk.to_vec());
        }
      }
    }
  }
}

#[derive(Debug)]
struct StatefulLedDeclaration {
  active: bool,
  color: Color,
  owning_system: u64,
  z_index: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DeclarationIdentifier {
  system_id: u64,
  z_index: i8,
}

#[derive(Debug, Clone)]
pub enum LedConflictResolution {
  /// Alternate between conflicting systems every N milliseconds (250ms)
  Alternate,
  /// Mix colors from conflicting systems, e.g. red and blue set on the same LED renders as purple
  Mix,
  /// The first system to make a declaration wins (default)
  FirstWins,
}

#[cfg(test)]
mod tests {
  use super::*;

  // TODO
}
