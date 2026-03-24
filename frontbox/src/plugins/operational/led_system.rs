use std::collections::HashMap;

use fast_protocol::Color;

use crate::prelude::*;

/// A system for merging the state of the machine's LEDs. While it's possible to invoke a command directly to forcibly
/// set the state of an LED, this approach ensures resolution amongst multiple competing systems. Typically this would
/// be used for shared things like lanes and target LEDs.
pub struct LedDeclarationSystem {
  declarations: HashMap<u64, HashMap<&'static str, Color>>,
  overrides: HashMap<u64, HashMap<&'static str, (Color, LedOverrideType)>>,
  alternator: AlternateResolver,
}

impl LedDeclarationSystem {
  pub fn new() -> Self {
    Self {
      declarations: HashMap::new(),
      overrides: HashMap::new(),
      alternator: AlternateResolver::new(),
    }
  }

  fn declare_led(&mut self, system_id: u64, led_name: &'static str, color: Color) {
    self
      .declarations
      .entry(system_id)
      .or_insert_with(HashMap::new)
      .insert(led_name, color);
  }

  fn declare_led_override(
    &mut self,
    system_id: u64,
    led_name: &'static str,
    color: Color,
    override_type: LedOverrideType,
  ) {
    self
      .overrides
      .entry(system_id)
      .or_insert_with(HashMap::new)
      .insert(led_name, (color, override_type));
  }
}

impl System for LedDeclarationSystem {
  fn on_startup(&mut self, ctx: &mut Context) {
    ctx.register_command::<DeclareLed>();
  }

  fn on_event(&mut self, _event: &dyn Signal, ctx: &mut Context) {
    if let Some(_) = _event.downcast_ref::<ExpansionNetworkReset>() {
      // reset state of LEDs in store
      let led_lookup = ctx.expect_mut::<LedLookup>();
      for addressable_led in led_lookup.values_mut() {
        addressable_led.color = Color::off();
      }
    }
  }

  fn on_tick(&mut self, delta: Duration, ctx: &mut Context) {
    self.alternator.tick(delta);
    let system_state = ctx.expect::<SystemState>();

    // group declarations by LED name, finding conflicts
    let mut conflicts: HashMap<&'static str, Vec<(u64, Color)>> = HashMap::new();
    let mut led_temp_updates: HashMap<&'static str, (u64, Color)> = HashMap::new();

    for (system_id, colors) in &self.declarations {
      if system_state.is_active(*system_id) {
        for (led_name, color) in colors {
          if let Some(conflict_list) = conflicts.get_mut(led_name) {
            conflict_list.push((*system_id, *color));
          } else if led_temp_updates.contains_key(led_name) {
            let current = led_temp_updates.remove(led_name).unwrap();
            conflicts.insert(led_name, vec![current, (*system_id, *color)]);
          } else {
            led_temp_updates.insert(led_name, (*system_id, *color));
          }
        }
      }
    }

    // resolve conflicts
    for (led_name, conflict_list) in conflicts {
      let resolved = self.alternator.resolve(led_name, conflict_list);
      led_temp_updates.insert(led_name, (0, resolved));
    }

    // TODO: apply overrides

    // diff with current state
    let mut led_updates: Vec<AddressableLed> = Vec::new();
    let led_lookup = ctx.expect_mut::<LedLookup>();
    for (led_name, (_, new_color)) in led_temp_updates {
      if let Some(addressable_led) = led_lookup.get_mut(led_name) {
        if addressable_led.color != new_color {
          addressable_led.color = new_color;
          led_updates.push(addressable_led.clone());
        }
      }
    }

    if !led_updates.is_empty() {
      ctx.command(SetLedBulk(led_updates));
    }
  }

  fn on_command(&mut self, command: &dyn Signal, caller_id: u64, _ctx: &mut Context) {
    if let Some(declaration) = command.downcast_ref::<DeclareLed>() {
      self.declare_led(caller_id, declaration.0, declaration.1);
    } else if let Some(cmd) = command.downcast_ref::<UndeclareLed>() {
      self.declare_led(caller_id, cmd.0, Color::off());
    } else if let Some(cmd) = command.downcast_ref::<DeclareLedOverride>() {
      self.declare_led_override(caller_id, cmd.0, cmd.1, cmd.2);
    } else if let Some(cmd) = command.downcast_ref::<UndeclareLedOverride>() {
      if let Some(overrides) = self.overrides.get_mut(&caller_id) {
        overrides.remove(cmd.0);
      }
      // check if there are existing declarations for this same LED and if not set it to off
      if let Some(declarations) = self.declarations.get(&caller_id) {
        if !declarations.contains_key(cmd.0) {
          // set this to off to clear out any override that might have been in place
          self.declare_led(caller_id, cmd.0, Color::off());
        }
      }
    }
  }
}

pub struct DeclareLed(
  /// LED name
  pub &'static str,
  pub Color,
);

pub struct UndeclareLed(pub &'static str);

pub struct DeclareLedOverride(
  /// LED name
  pub &'static str,
  pub Color,
  pub LedOverrideType,
);

pub struct UndeclareLedOverride(pub &'static str);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Default)]
pub enum LedOverrideType {
  /// Only the overridden LED state will be shown
  #[default]
  Opaque,
  /// Overridden LED color will be mixed with the underlying color
  Mix,
}
