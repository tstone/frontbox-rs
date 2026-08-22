use std::collections::HashMap;
use std::i8;

use image::{Pixel, Rgba};
use itertools::Itertools;

use crate::prelude::*;

const LED_SET_BATCH_SIZE: usize = 24;

/// # LedSystem
///
/// ### Features
/// - **Conflict Resolution** -- Multiple systems can declare a color on the same layer for an LED and the LedSystem will handle resolving that conflict automatically (conflict resolution mode is user settable)
/// - **Layer (z-index) Support** -- It's possible to keep an "under layer" active while playing temporary animations a layer above
/// - **Alpha Compositing** -- LEDs are rendered in RGBA which supports transparency (under colors show through partially)
/// - An easy way to de-activate LED declarations when a system is de-activated
/// - Automatic clearing of unset LEDs per frame
///
/// ### In Use
///
/// Using the LedSystems works by way of a _declaration_. A declaration doesn't forcibly set an LED, instead it's more like a request, "Hello, I am system 12345 and would prefer for this LED to be this color at this level of priority" (you can think of Z-index layers as levels of priority). Each render frame, the LedSystem looks through all active declarations, chooses the highest priority one, resolves any conflicting declarations, and updates the state of LEDs that need to change. This process also detects LEDs that are no longer set and clears them automatically.
///
/// ```rust
/// // declare LEDs by name...
/// ctx.declare_leds(
///   &leds::EXAMPLE.q().at_z(3),
///   ColorSequence::solid(Rgba::yellow())
/// );
///
/// // ...or by group
/// ctx.declare_leds(
///   vec![&leds::EX1.q(), &leds::EX2.q(), &leds::EX3.q()],
///   ColorSequence::gradient(vec![Rgba::red(), Rgba::yellow()])
/// );
/// ```
///
/// `declare_leds` takes a `HardwareQuery`, which is the reason for `.q()`. More about this later.
///
/// Later on if these declarations need to be temporarily suspended because the System is going inactive, they can be temporarily disabled:
///
/// ```rust
/// ctx.deactivate_led_declarations();
/// ```
///
/// In fact, this behavior is built-in to `System` by default. When a system goes inactive, if `LedSystem` is live, it will de-activate declarations, then re-activate them once the System comes back.
///
/// ### Layering
///
/// It is possible to declare multiple layers for the same LED. If higher layers are opaque they will be rendered. If higher layers are transparent, they will render with a degree of "see-through" to layers below them.
///
/// ```rust
/// // higher layer declares 50% transparent red
/// ctx.declare_leds(
///   &leds::EXAMPLE.q().at_z(1),
///   ColorSequence::solid(Rgba::red().with_alpha_f32(0.5))
/// );
///
/// // over top of white
/// ctx.declare_leds(&leds::EXAMPLE.q(), ColorSequence::solid(Rgba::white()));
///
/// // final color renders as pink [255, 127, 127, 255]
/// ```

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

  pub(crate) fn declarations_for(&self, addr: &LedAddress) -> Vec<StatefulLedDeclaration> {
    self
      .declarations
      .get(addr)
      .map(|ds| ds.values().copied().collect())
      .unwrap_or(Vec::new())
  }

  pub fn reset(&mut self) {
    self.declarations.clear();
    self.prior_render.clear();
    self.conflict_resolution.clear();
    self.alternate_resolver.reset();
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
    log::trace!(target: "frontbox::leds", "LED declarations declared: {:?}", declarations);

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

  /// Entirely remove all declarations by the given system
  pub fn undeclare_by_system(&mut self, system_id: &u64) {
    for declarations in self.declarations.values_mut() {
      declarations.retain(|id, _| &id.system_id != system_id);
    }
  }

  /// Keep declarations but mark them as inactive so they don't render
  pub fn deactivate_by_system(&mut self, system_id: u64) {
    log::info!(target: "frontbox::leds", "Deactivating all LED declarations for {}", system_id);
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
      .filter(|(id, dec)| id.z_index == *max_z && dec.active)
      .collect::<Vec<_>>();

    let top_color = if top_declarations.len() == 1 {
      top_declarations[0].1.color
    } else if top_declarations.len() > 1 {
      let resolution_strategy = conflict_resolution
        .get(led)
        .unwrap_or(&LedConflictResolution::FirstWins);

      log::debug!(
        target: "frontbox::leds",
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
      // TODO: remove inactive indexes (here or above?)
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
  fn on_spawn(&mut self, ctx: &SystemContext) {
    // Create a copy of all LEDs addresses to reference during rendering
    self.all_addresses = ctx.leds.values().map(|led| led.address.clone()).collect();
  }

  fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
    if let Some(SystemDespawned { id, .. }) = event.downcast_ref::<SystemDespawned>() {
      self.undeclare_by_system(id);
    }
  }

  fn on_tick(&mut self, delta: Duration, _ctx: &SystemContext) {
    self.alternate_resolver.accumulate(delta);
  }

  fn on_render(&mut self, ctx: &SystemContext) {
    let mut leds_to_set: Vec<(LedAddress, Rgba<u8>)> = Vec::new();

    for led in self.all_addresses.iter() {
      if let Some(declarations) = self.declarations.get(led) {
        // assemble a list of unique z-indexes which are defined for this LED
        let z_indexes = declarations
          .iter()
          .filter(|(_, d)| d.active)
          .map(|(id, _)| id.z_index)
          .sorted()
          .unique()
          .collect();

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
        log::trace!(target: "frontbox::leds", "Turning off LED at {:?}", led);
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
      log::debug!(target: "frontbox::leds", "Setting LED at {:?} to {:?}", led, color);
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

      let machine = ctx.expect::<Machine>();
      for (address, leds) in outgoing.into_iter() {
        for chunk in leds.chunks(LED_SET_BATCH_SIZE) {
          machine.set_leds(address.board_address, address.breakout, chunk.to_vec());
        }
      }
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub struct StatefulLedDeclaration {
  pub active: bool,
  pub color: Rgba<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeclarationIdentifier {
  pub system_id: u64,
  pub z_index: i8,
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
  fn declare_and_undeclare_systems() {
    let mut system = LedSystem::new();
    let led = LedAddress {
      exp: ExpAddress::new(3, None, 0),
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
  fn declare_overwrite() {
    let mut system = LedSystem::new();
    let led = LedAddress {
      exp: ExpAddress::new(3, None, 0),
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
  fn declare_and_undeclare_multiple() {
    let mut system = LedSystem::new();
    let led1 = LedAddress {
      exp: ExpAddress::new(3, None, 0),
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
  fn declare_and_undeclare_z_index() {
    let mut system = LedSystem::new();
    let led = LedAddress {
      exp: ExpAddress::new(3, None, 0),
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
  fn alpha_fall_through_ignores_disabled() {
    let led = LedAddress {
      exp: ExpAddress::new(3, None, 0),
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
          color: Rgba::default(),
        },
      ),
      (
        DeclarationIdentifier {
          system_id: 43,
          z_index: 0,
        },
        StatefulLedDeclaration {
          active: false, // final color should NOT be red
          color: Rgba::red(),
        },
      ),
    ]);

    let conflict_resolution = HashMap::new();
    let mut alternate_resolver = AlternateResolver::new();

    let resolved_color = LedSystem::resolve_led_color(
      &led,
      vec![0, 1],
      &declarations,
      &conflict_resolution,
      &mut alternate_resolver,
    );

    // final color should be clear since system 11 declarations are inactive
    assert_eq!(resolved_color, Rgba::default());
  }

  #[test]
  fn resolve_color() {
    let led = LedAddress {
      exp: ExpAddress::new(3, None, 0),
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
  fn resolve_color_conflict() {
    let led = LedAddress {
      exp: ExpAddress::new(3, None, 0),
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
  fn resolve_color_alpha_compositing() {
    let led = LedAddress {
      exp: ExpAddress::new(3, None, 0),
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
