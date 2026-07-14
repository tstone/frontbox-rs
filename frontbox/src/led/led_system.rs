use std::collections::HashMap;
use std::i8;

use image::{Pixel, Rgba};
use itertools::Itertools;

use crate::prelude::*;

const LED_SET_BATCH_SIZE: usize = 24;
pub struct LedSystem {
  all_addresses: Vec<LedAddress>,
  // Rule: Systems cannot contradict themselves. Declarations are thus unique by led/system/z-index.
  declarations: HashMap<LedAddress, HashMap<DeclarationIdentifier, StatefulLedDeclaration>>,
  prior_render: HashMap<LedAddress, Rgba<u8>>,
  conflict_resolution: HashMap<LedAddress, LedConflictResolution>,
  alternate_resolver: AlternateResolver,
}

impl LedSystem {
  pub fn new() -> Self {
    Self {
      declarations: HashMap::new(),
      all_addresses: Vec::new(),
      prior_render: HashMap::new(),
      conflict_resolution: HashMap::new(),
      alternate_resolver: AlternateResolver::new(),
    }
  }

  /// Declare that a system wants to set a LED to a color. Handles resolution and rendering.
  pub fn declare<'a>(&mut self, owning_system: u64, declarations: impl Into<LedDeclarations<'a>>) {
    self.declare_inner(owning_system, declarations, true);
  }

  /// Same as declare but doesn't render until activate_by_system is called. Useful for systems that want to prepare declarations in advance and activate them all at once later.
  pub fn declare_inactive<'a>(
    &mut self,
    owning_system: u64,
    declarations: impl Into<LedDeclarations<'a>>,
  ) {
    self.declare_inner(owning_system, declarations, false);
  }

  fn declare_inner<'a>(
    &mut self,
    owning_system: u64,
    declarations: impl Into<LedDeclarations<'a>>,
    active: bool,
  ) {
    let declarations: LedDeclarations = declarations.into();
    for (led, color) in declarations.pairings {
      let declaration = StatefulLedDeclaration { active, color };
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

  fn resolve_led_color(
    led: &LedAddress,
    mut z_indexes: Vec<i8>,
    declarations: &HashMap<DeclarationIdentifier, StatefulLedDeclaration>,
    conflict_resolution: &HashMap<LedAddress, LedConflictResolution>,
    alternate_resolver: &mut AlternateResolver,
  ) -> Rgba<u8> {
    let max_z = z_indexes.iter().max().unwrap_or(&i8::MIN);
    let top_declarations = declarations
      .iter()
      .filter(|(id, _)| id.z_index == *max_z)
      .collect::<Vec<_>>();

    let top_color = if top_declarations.len() == 1 {
      top_declarations[0].1.color
    } else if top_declarations.len() > 1 {
      let resolution_strategy = conflict_resolution
        .get(led)
        .unwrap_or(&LedConflictResolution::FirstWins);

      log::trace!(
        "Resolving LED declaration conflict on {:?} with {:?}",
        led,
        resolution_strategy
      );

      match resolution_strategy {
        LedConflictResolution::FirstWins => top_declarations[0].1.color,
        LedConflictResolution::Mix => top_declarations
          .iter()
          .map(|(_, d)| d.color)
          .reduce(|acc, c| acc.mix_with(c, 0.5))
          .unwrap_or(Rgba::default()),
        LedConflictResolution::Alternate => {
          let colors = top_declarations.iter().map(|(_, d)| d.color).collect();
          alternate_resolver.resolve(led.clone(), colors)
        }
      }
    } else {
      Rgba::default()
    };

    // if the top color is transparent, composite it with the next highest declaration below it
    if top_color.alpha() < 255 && z_indexes.len() > 1 {
      z_indexes.pop();
      let next_color = Self::resolve_led_color(
        led,
        z_indexes,
        declarations,
        conflict_resolution,
        alternate_resolver,
      );
      top_color.composite_over(next_color)
    } else {
      top_color
    }
  }
}

impl System for LedSystem {
  fn on_spawn(&mut self, ctx: &Context) {
    // Create a copy of all LEDs addresses to reference during rendering
    self.all_addresses = ctx.leds.values().map(|led| led.address.clone()).collect();
  }

  fn on_tick(&mut self, delta: Duration, _ctx: &Context) {
    self.alternate_resolver.accumulate(delta);
  }

  fn on_render(&mut self, ctx: &Context) {
    let mut leds_to_set: Vec<(LedAddress, Rgba<u8>)> = Vec::new();

    for led in self.all_addresses.iter() {
      if let Some(declarations) = self.declarations.get(led) {
        // take only active, highest z-index declaration for each LED
        let active = declarations.iter().filter(|(_, d)| d.active);
        // assemble a list of unique z-indexes defined for this LED
        let z_indexes =
          active
            .clone()
            .map(|(id, _)| id.z_index)
            .sorted()
            .fold(Vec::new(), |mut acc, z| {
              if !acc.contains(&z) {
                acc.push(z);
              }
              acc
            });

        let final_color = Self::resolve_led_color(
          led,
          z_indexes,
          declarations,
          &self.conflict_resolution,
          &mut self.alternate_resolver,
        );
        leds_to_set.push((led.clone(), final_color));
      } else {
        // no declarations for this LED = turn it off
        // if it's already off this will get filtered out below
        log::trace!("Turning off LED at {:?}", led);
        leds_to_set.push((led.clone(), Rgba::default()));
      }
    }

    // filter out LEDs that are the same as the prior render to avoid redundant updates
    // "there are hard limits to the amount of data that can go out over the EXP bus of about 80 characters per millisecond" -- ecurtz
    leds_to_set.retain(|(led, color)| {
      if let Some(prior_color) = self.prior_render.get(led) {
        if prior_color == color {
          log::trace!(
            "Skipping LED at {:?} because no change ({:?} vs {:?})",
            led,
            prior_color,
            color
          );
          return false;
        }
      }
      self.prior_render.insert(led.clone(), *color);
      log::trace!("Setting LED at {:?} to {:?}", led, color);
      true
    });

    if leds_to_set.len() > 0 {
      // group by address to send to machine
      let outgoing: HashMap<ExpAddress, Vec<(u16, Rgba<u8>)>> =
        leds_to_set
          .into_iter()
          .fold(HashMap::new(), |mut acc, (addr, color)| {
            let channels = ctx.leds.color_channels_by_id(&addr);
            let remapped_color = color.remap(channels);
            acc
              .entry(addr.exp)
              .or_insert_with(Vec::new)
              .push((addr.index, remapped_color));
            acc
          });

      let machine = ctx.systems.expect::<Machine>();
      for (address, leds) in outgoing.into_iter() {
        for chunk in leds.chunks(LED_SET_BATCH_SIZE) {
          machine.set_leds(address.board_address, address.breakout, chunk.to_vec());
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

  // Two different systems declare the same LED
  #[test]
  fn test_declare_and_undeclare_systems() {
    let mut system = LedSystem::new();
    let led = LedAddress {
      exp: ExpAddress {
        board_address: 3,
        breakout: None,
        port: 0,
      },
      index: 1,
    };

    system.declare(
      42,
      LedDeclarations::new(vec![(&led.clone(), Rgba::red())], 0),
    );
    assert!(system.declarations.get(&led).unwrap().len() == 1);
    system.declare(
      43,
      LedDeclarations::new(vec![(&led.clone(), Rgba::red())], 0),
    );
    assert!(system.declarations.get(&led).unwrap().len() == 2);

    system.undeclare(42, LedIdentifications::new(vec![led.clone()], 0));

    // the declaration for system 43 should still be there
    assert!(system.declarations.get(&led).unwrap().len() == 1);
  }

  // One system declares multiple times to overwrite
  #[test]
  fn test_declare_overwrite() {
    let mut system = LedSystem::new();
    let led = LedAddress {
      exp: ExpAddress {
        board_address: 3,
        breakout: None,
        port: 0,
      },
      index: 1,
    };

    system.declare(
      42,
      LedDeclarations::new(vec![(&led.clone(), Rgba::red())], 0),
    );
    assert_eq!(system.declarations.get(&led).unwrap().len(), 1);
    system.declare(
      42,
      LedDeclarations::new(vec![(&led.clone(), Rgba::blue())], 0),
    );

    let final_map = system.declarations.get(&led).unwrap();
    assert_eq!(final_map.len(), 1);

    let final_dec = final_map.values().into_iter().next().unwrap();
    assert_eq!(final_dec.color, Rgba::blue());
  }

  // One system declares two different LEDs
  #[test]
  fn test_declare_and_undeclare_multiple() {
    let mut system = LedSystem::new();
    let led1 = LedAddress {
      exp: ExpAddress {
        board_address: 3,
        breakout: None,
        port: 0,
      },
      index: 1,
    };
    let led2 = LedAddress {
      exp: ExpAddress {
        board_address: 3,
        breakout: None,
        port: 0,
      },
      index: 2,
    };

    system.declare(
      42,
      LedDeclarations::new(vec![(&led1.clone(), Rgba::red())], 0),
    );
    system.declare(
      42,
      LedDeclarations::new(vec![(&led2.clone(), Rgba::red())], 0),
    );
    assert_eq!(system.declarations.get(&led1).unwrap().len(), 1);
    assert_eq!(system.declarations.get(&led2).unwrap().len(), 1);

    system.undeclare(42, LedIdentifications::new(vec![led1.clone()], 0));

    // the declaration for led2 should still be there
    assert_eq!(system.declarations.get(&led2).unwrap().len(), 1);
  }

  // One system declares the same LED at two different z-indexes
  #[test]
  fn test_declare_and_undeclare_z_index() {
    let mut system = LedSystem::new();
    let led = LedAddress {
      exp: ExpAddress {
        board_address: 3,
        breakout: None,
        port: 0,
      },
      index: 1,
    };

    system.declare(
      42,
      LedDeclarations::new(vec![(&led.clone(), Rgba::red())], 1),
    );
    system.declare(
      42,
      LedDeclarations::new(vec![(&led.clone(), Rgba::blue())], 2),
    );
    assert!(system.declarations.get(&led).unwrap().len() == 2);

    system.undeclare(42, LedIdentifications::new(vec![led.clone()], 1));

    // the declaration for z-index 2 should still be there
    assert!(system.declarations.get(&led).unwrap().len() == 1);
  }

  #[test]
  fn test_resolve_color() {
    let led = LedAddress {
      exp: ExpAddress {
        board_address: 3,
        breakout: None,
        port: 0,
      },
      index: 1,
    };

    let declarations = HashMap::from([
      (
        DeclarationIdentifier {
          system_id: 42,
          z_index: 0,
        },
        StatefulLedDeclaration {
          active: true,
          color: Rgba::red(),
        },
      ),
      (
        DeclarationIdentifier {
          system_id: 43,
          z_index: 1, // higher index, blue should win
        },
        StatefulLedDeclaration {
          active: true,
          color: Rgba::blue(),
        },
      ),
    ]);

    let conflict_resolution = HashMap::new();
    let mut alternate_resolver = AlternateResolver::new();

    let resolved_color = LedSystem::resolve_led_color(
      &led,
      vec![1],
      &declarations,
      &conflict_resolution,
      &mut alternate_resolver,
    );

    assert_eq!(resolved_color, Rgba([0, 0, 255, 255]));
  }

  #[test]
  fn test_resolve_color_conflict() {
    let led = LedAddress {
      exp: ExpAddress {
        board_address: 3,
        breakout: None,
        port: 0,
      },
      index: 1,
    };

    let declarations = HashMap::from([
      (
        DeclarationIdentifier {
          system_id: 42,
          z_index: 0,
        },
        StatefulLedDeclaration {
          active: true,
          color: Rgba::red(),
        },
      ),
      (
        DeclarationIdentifier {
          system_id: 43,
          z_index: 0,
        },
        StatefulLedDeclaration {
          active: true,
          color: Rgba::blue(),
        },
      ),
    ]);

    let conflict_resolution = HashMap::from([(
      led.clone(),
      LedConflictResolution::Mix, // red and blue should mix to purple
    )]);
    let mut alternate_resolver = AlternateResolver::new();

    let resolved_color = LedSystem::resolve_led_color(
      &led,
      vec![0],
      &declarations,
      &conflict_resolution,
      &mut alternate_resolver,
    );

    assert_eq!(resolved_color, Rgba([127, 0, 127, 255]));
  }

  // alpha compositing test - top semi-transparent declaration should composite with the next highest declaration below it
  #[test]
  fn test_resolve_color_alpha_compositing() {
    let led = LedAddress {
      exp: ExpAddress {
        board_address: 3,
        breakout: None,
        port: 0,
      },
      index: 1,
    };

    let declarations = HashMap::from([
      (
        DeclarationIdentifier {
          system_id: 42,
          z_index: 1,
        },
        StatefulLedDeclaration {
          active: true,
          color: Rgba([255, 0, 0, 127]), // semi-transparent red
        },
      ),
      (
        DeclarationIdentifier {
          system_id: 43,
          z_index: 0,
        },
        StatefulLedDeclaration {
          active: true,
          color: Rgba([255, 255, 255, 255]), // opaque white
        },
      ),
    ]);

    let conflict_resolution = HashMap::from([(
      led.clone(),
      LedConflictResolution::Mix, // red and blue should mix to purple
    )]);
    let mut alternate_resolver = AlternateResolver::new();

    let resolved_color = LedSystem::resolve_led_color(
      &led,
      vec![0, 1],
      &declarations,
      &conflict_resolution,
      &mut alternate_resolver,
    );

    // final color shows through as pink
    assert_eq!(resolved_color, Rgba([255, 127, 127, 255]));
  }
}
