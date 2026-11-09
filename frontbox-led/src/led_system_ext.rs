use frontbox_core::prelude::*;
use frontbox_gfx::prelude::*;

use crate::*;

pub trait LedSystemExt {
  /// Declare one color for every LED the targets resolve to
  fn declare_leds<T: Contextual<LedIdentifications>>(&self, targets: &T, color: Rgba<u8>);
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

impl<'a> LedSystemExt for SystemContext<'a> {
  fn declare_leds<T: Contextual<LedIdentifications>>(&self, targets: &T, color: Rgba<u8>) {
    self.declare_led_colors(targets, |count| vec![color; count]);
  }

  fn declare_led_colors<T: Contextual<LedIdentifications>>(
    &self,
    targets: &T,
    colors: impl FnOnce(usize) -> Vec<Rgba<u8>>,
  ) {
    with_led_system(self, |led_system| {
      let targets = targets.resolve(&self);
      let colors = colors(targets.leds.len());
      let declarations = LedDeclarations {
        pairings: targets.leds.iter().zip(colors).collect(),
        z_index: targets.z_index,
      };
      led_system.declare(self.id(), declarations);
    });
  }

  fn declare_leds_inactive<T: Contextual<LedIdentifications>>(&self, targets: &T, color: Rgba<u8>) {
    with_led_system(self, |led_system| {
      let targets = targets.resolve(&self);
      let colors = vec![color; targets.leds.len()];
      let declarations = LedDeclarations {
        pairings: targets.leds.iter().zip(colors).collect(),
        z_index: targets.z_index,
      };
      led_system.declare_inactive(self.id(), declarations);
    });
  }

  fn undeclare_leds<T: Contextual<LedIdentifications>>(&self, targets: &T) {
    with_led_system(self, |led_system| {
      let targets = targets.resolve(&self);
      led_system.undeclare(self.id(), targets);
    });
  }

  fn deactivate_led_declarations(&self) {
    with_led_system(self, |led_system| {
      led_system.deactivate_by_system(self.id());
    });
  }

  fn activate_led_declarations(&self) {
    with_led_system(self, |led_system| {
      led_system.activate_by_system(self.id());
    });
  }

  fn set_led_conflict_resolution<T: Contextual<LedIdentifications>>(
    &self,
    targets: &T,
    resolution: LedConflictResolution,
  ) {
    with_led_system(self, |led_system| {
      let targets = targets.resolve(&self);
      led_system.set_conflict_resolution(targets, resolution);
    });
  }
}

fn with_led_system<T>(ctx: &SystemContext, f: impl FnOnce(&mut LedSystem) -> T) {
  if let Some(mut led_system) = ctx.systems.get::<LedSystem>() {
    f(&mut led_system);
  } else {
    log::error!("LedSystem not running.");
  }
}
