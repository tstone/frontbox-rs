use crate::prelude::*;

pub trait LedSystemExt {
  fn declare_leds<T: Contextual<LedIdentifications>>(&self, targets: &T, colors: ColorSequence);
  fn declare_leds_inactive<T: Contextual<LedIdentifications>>(
    &self,
    targets: &T,
    colors: ColorSequence,
  );
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
  fn declare_leds<T: Contextual<LedIdentifications>>(&self, targets: &T, seq: ColorSequence) {
    with_led_system(self, |led_system| {
      let targets = targets.resolve(&self);
      let colors = seq.generate(targets.leds.len());
      let declarations = LedDeclarations {
        pairings: targets.leds.iter().zip(colors).collect(),
        z_index: targets.z_index,
      };
      led_system.declare(self.current_system_id(), declarations);
    });
  }

  fn declare_leds_inactive<T: Contextual<LedIdentifications>>(
    &self,
    targets: &T,
    seq: ColorSequence,
  ) {
    with_led_system(self, |led_system| {
      let targets = targets.resolve(&self);
      let colors = seq.generate(targets.leds.len());
      let declarations = LedDeclarations {
        pairings: targets.leds.iter().zip(colors).collect(),
        z_index: targets.z_index,
      };
      led_system.declare_inactive(self.current_system_id(), declarations);
    });
  }

  fn undeclare_leds<T: Contextual<LedIdentifications>>(&self, targets: &T) {
    with_led_system(self, |led_system| {
      let targets = targets.resolve(&self);
      led_system.undeclare(self.current_system_id(), targets);
    });
  }

  fn deactivate_led_declarations(&self) {
    with_led_system(self, |led_system| {
      led_system.deactivate_by_system(self.current_system_id());
    });
  }

  fn activate_led_declarations(&self) {
    with_led_system(self, |led_system| {
      led_system.activate_by_system(self.current_system_id());
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
  if let Some(mut led_system) = ctx.get::<LedSystem>() {
    f(&mut led_system);
  } else {
    log::error!("LedSystem not running.");
  }
}
