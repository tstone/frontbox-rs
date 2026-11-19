use std::cell::RefMut;

use frontbox_core::prelude::*;
use frontbox_gfx::prelude::*;

use crate::*;

pub trait LightManagerExt {
  /// The running light management, for calls not covered below. `None` if it isn't running, or is in use.
  fn light_manager(&self) -> Option<RefMut<'_, dyn LightManager>>;

  /// Declare one color for every LED the targets resolve to
  fn declare_leds<T: Contextual<LedIdentifications>>(&self, targets: &T, color: Rgba<u8>);
  /// Declare one color for every LED the targets resolve to, flashing on and off
  fn declare_leds_flashing<T: Contextual<LedIdentifications>>(&self, targets: &T, color: Rgba<u8>);
  /// Declare colors for the targets, given by `colors` from the number of LEDs they resolve to
  fn declare_led_colors<T: Contextual<LedIdentifications>>(
    &self,
    targets: &T,
    colors: impl FnOnce(usize) -> Vec<Rgba<u8>>,
  );
  fn declare_leds_inactive<T: Contextual<LedIdentifications>>(&self, targets: &T, color: Rgba<u8>);
  fn undeclare_leds<T: Contextual<LedIdentifications>>(&self, targets: &T);
  fn activate_led_declarations(&self);
  fn deactivate_led_declarations(&self);
  fn set_led_conflict_resolution<T: Contextual<LedIdentifications>>(
    &self,
    targets: &T,
    resolution: LedConflictResolution,
  );
}

impl<'a> LightManagerExt for SystemContext<'a> {
  fn light_manager(&self) -> Option<RefMut<'_, dyn LightManager>> {
    self.systems.get_dyn::<dyn LightManager>()
  }

  fn declare_leds<T: Contextual<LedIdentifications>>(&self, targets: &T, color: Rgba<u8>) {
    self.declare_led_colors(targets, |count| vec![color; count]);
  }

  fn declare_leds_flashing<T: Contextual<LedIdentifications>>(&self, targets: &T, color: Rgba<u8>) {
    with_light_manager(self, |manager| {
      let targets = targets.resolve(&self);
      let pairings = targets.leds.iter().map(|led| (led, color)).collect();
      manager.declare(self.id(), LedDeclarations::new(pairings, targets.z_index).flashing());
    });
  }

  fn declare_led_colors<T: Contextual<LedIdentifications>>(
    &self,
    targets: &T,
    colors: impl FnOnce(usize) -> Vec<Rgba<u8>>,
  ) {
    with_light_manager(self, |manager| {
      let targets = targets.resolve(&self);
      let colors = colors(targets.leds.len());
      let declarations = LedDeclarations::new(targets.leds.iter().zip(colors).collect(), targets.z_index);
      manager.declare(self.id(), declarations);
    });
  }

  fn declare_leds_inactive<T: Contextual<LedIdentifications>>(&self, targets: &T, color: Rgba<u8>) {
    with_light_manager(self, |manager| {
      let targets = targets.resolve(&self);
      let pairings = targets.leds.iter().map(|led| (led, color)).collect();
      manager.declare_inactive(self.id(), LedDeclarations::new(pairings, targets.z_index));
    });
  }

  fn undeclare_leds<T: Contextual<LedIdentifications>>(&self, targets: &T) {
    with_light_manager(self, |manager| {
      let targets = targets.resolve(&self);
      manager.undeclare(self.id(), targets.into());
    });
  }

  fn deactivate_led_declarations(&self) {
    with_light_manager(self, |manager| {
      manager.deactivate_by_system(self.id());
    });
  }

  fn activate_led_declarations(&self) {
    with_light_manager(self, |manager| {
      manager.activate_by_system(self.id());
    });
  }

  fn set_led_conflict_resolution<T: Contextual<LedIdentifications>>(
    &self,
    targets: &T,
    resolution: LedConflictResolution,
  ) {
    with_light_manager(self, |manager| {
      let targets = targets.resolve(&self);
      manager.set_conflict_resolution(targets.into(), resolution);
    });
  }
}

fn with_light_manager<T>(ctx: &SystemContext, f: impl FnOnce(&mut dyn LightManager) -> T) {
  if let Some(mut manager) = ctx.light_manager() {
    f(&mut *manager);
  } else {
    log::error!(target: "frontbox::led", "Light management is not running.");
  }
}
