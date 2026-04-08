use std::collections::HashMap;
use std::i8;

use image::Rgba;

use crate::prelude::*;

const LED_SET_BATCH_SIZE: usize = 24;
pub struct LedSystem {
  all_leds: Vec<AddressableLed>,
  // Rule: Systems cannot contradict themselves. Declarations are thus unique by led/system/z-index.
  declarations: HashMap<AddressableLed, HashMap<DeclarationIdentifier, StatefulLedDeclaration>>,
  prior_render: HashMap<AddressableLed, Rgba<u8>>,
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

  /// Declare that a system wants to set a LED to a color. Handles resolution and rendering.
  pub fn declare(&mut self, owning_system: u64, declarations: impl Into<LedDeclarations>) {
    self.declare_inner(owning_system, declarations, true);
  }

  /// Same as declare but doesn't render until activate_by_system is called. Useful for systems that want to prepare declarations in advance and activate them all at once later.
  pub fn declare_inactive(&mut self, owning_system: u64, declarations: impl Into<LedDeclarations>) {
    self.declare_inner(owning_system, declarations, false);
  }

  fn declare_inner(
    &mut self,
    owning_system: u64,
    declarations: impl Into<LedDeclarations>,
    active: bool,
  ) {
    let declarations: LedDeclarations = declarations.into();
    for (led, color) in declarations.pairings {
      // A color value of None is the same as undeclaring
      if color.is_none() {
        self.undeclare(owning_system, (led, Some(declarations.z_index)));
        continue;
      }

      let declaration = StatefulLedDeclaration {
        active,
        color: color.unwrap(),
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
    let target_id = DeclarationIdentifier {
      system_id,
      z_index: identifications.z_index,
    };

    for led in identifications.leds {
      if let Some(declarations) = self.declarations.get_mut(&led) {
        declarations.retain(|id, _| id != &target_id);
        if declarations.is_empty() {
          self.declarations.remove(&led);
        }
      }
    }
  }

  /// Keep declarations but mark them as inactive so they don't render
  pub fn deactivate_by_system(&mut self, system_id: u64) {
    for declarations in self.declarations.values_mut() {
      for (id, declaration) in declarations.iter_mut() {
        if id.system_id == system_id {
          declaration.active = false;
        }
      }
    }
  }

  /// Mark any existing declarations as active
  pub fn activate_by_system(&mut self, system_id: u64) {
    for declarations in self.declarations.values_mut() {
      for (id, declaration) in declarations.iter_mut() {
        if id.system_id == system_id {
          declaration.active = true;
        }
      }
    }
  }

  pub fn set_conflict_resolution(
    &mut self,
    identifications: impl Into<LedIdentifications>,
    resolution: LedConflictResolution,
  ) {
    let identifications = identifications.into();
    for led in identifications.leds {
      self.conflict_resolution.insert(led.clone(), resolution);
    }
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
    let mut leds_to_set: Vec<(AddressableLed, Rgba<u8>)> = Vec::new();
    for led in self.all_leds.iter() {
      if let Some(declarations) = self.declarations.get(led) {
        // take only active, highest z-index declaration for each LED
        let active = declarations.iter().filter(|(_, d)| d.active);
        let maz_z_index = active
          .clone()
          .map(|(id, _)| id.z_index)
          .max()
          .unwrap_or(i8::MIN);
        let top_declarations = active
          .filter(|(id, _)| id.z_index == maz_z_index)
          .collect::<Vec<_>>();

        if top_declarations.len() == 1 {
          leds_to_set.push((led.clone(), top_declarations[0].1.color));
        } else if top_declarations.len() > 1 {
          let resolution_strategy = self
            .conflict_resolution
            .get(led)
            .unwrap_or(&LedConflictResolution::FirstWins);
          log::trace!(
            "Resolving LED declaration conflict on {:?} with {:?}",
            led,
            resolution_strategy
          );

          match resolution_strategy {
            LedConflictResolution::FirstWins => {
              leds_to_set.push((led.clone(), top_declarations[0].1.color));
            }
            LedConflictResolution::Mix => {
              leds_to_set.push((
                led.clone(),
                top_declarations
                  .iter()
                  .map(|(_, d)| d.color)
                  .reduce(|acc, c| acc.mix_with(c, 0.5))
                  .unwrap_or(Rgba::default()),
              ));
            }
            LedConflictResolution::Alternate => {
              let colors = top_declarations.iter().map(|(_, d)| d.color).collect();
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
        leds_to_set.push((led.clone(), Rgba::default()));
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
      let outgoing: HashMap<LedAddress, Vec<(u16, Rgba<u8>)>> =
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
  color: Rgba<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DeclarationIdentifier {
  system_id: u64,
  z_index: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

  #[test]
  fn test_declare_and_undeclare_systems() {
    let mut system = LedSystem::new();
    let led = AddressableLed {
      address: LedAddress {
        address: 3,
        breakout: None,
        port: 0,
      },
      index: 1,
    };

    system.declare(
      42,
      LedDeclarations::new(vec![(led.clone(), Some(Rgba::red()))], 0),
    );
    assert!(system.declarations.get(&led).unwrap().len() == 1);
    system.declare(
      43,
      LedDeclarations::new(vec![(led.clone(), Some(Rgba::red()))], 0),
    );
    assert!(system.declarations.get(&led).unwrap().len() == 2);

    system.undeclare(42, LedIdentifications::new(vec![led.clone()], 0));

    // the declaration for system 43 should still be there
    assert!(system.declarations.get(&led).unwrap().len() == 1);
  }

  #[test]
  fn test_declare_and_undeclare_multiple() {
    let mut system = LedSystem::new();
    let led1 = AddressableLed {
      address: LedAddress {
        address: 3,
        breakout: None,
        port: 0,
      },
      index: 1,
    };
    let led2 = AddressableLed {
      address: LedAddress {
        address: 3,
        breakout: None,
        port: 0,
      },
      index: 2,
    };

    system.declare(
      42,
      LedDeclarations::new(vec![(led1.clone(), Some(Rgba::red()))], 0),
    );
    system.declare(
      42,
      LedDeclarations::new(vec![(led2.clone(), Some(Rgba::red()))], 0),
    );
    assert!(system.declarations.get(&led1).unwrap().len() == 2);

    system.undeclare(42, LedIdentifications::new(vec![led1.clone()], 0));

    // the declaration for led2 should still be there
    assert!(system.declarations.get(&led1).unwrap().len() == 1);
  }

  #[test]
  fn test_declare_and_undeclare_z_index() {
    let mut system = LedSystem::new();
    let led = AddressableLed {
      address: LedAddress {
        address: 3,
        breakout: None,
        port: 0,
      },
      index: 1,
    };

    system.declare(
      42,
      LedDeclarations::new(vec![(led.clone(), Some(Rgba::red()))], 1),
    );
    system.declare(
      42,
      LedDeclarations::new(vec![(led.clone(), Some(Rgba::blue()))], 2),
    );
    assert!(system.declarations.get(&led).unwrap().len() == 2);

    system.undeclare(42, LedIdentifications::new(vec![led.clone()], 1));

    // the declaration for z-index 2 should still be there
    assert!(system.declarations.get(&led).unwrap().len() == 1);
  }
}
