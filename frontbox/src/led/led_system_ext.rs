use crate::prelude::*;

pub trait LedSystemExt {
  fn declare_leds(&self, declarations: impl Into<LedDeclarations>);
  fn declare_leds_inactive(&self, declarations: impl Into<LedDeclarations>);
  fn undeclare_leds(&self, identifications: impl Into<LedIdentifications>);
  fn activate_led_declarations(&self);
  fn deactivate_led_declarations(&self);
  fn set_led_conflict_resolution(
    &self,
    identifications: impl Into<LedIdentifications>,
    resolution: LedConflictResolution,
  );
}

impl<'a> LedSystemExt for Context<'a> {
  fn declare_leds(&self, declarations: impl Into<LedDeclarations>) {
    with_led_system(self, |led_system| {
      led_system.declare(self.current_system_id(), declarations);
    });
  }

  fn declare_leds_inactive(&self, declarations: impl Into<LedDeclarations>) {
    with_led_system(self, |led_system| {
      led_system.declare_inactive(self.current_system_id(), declarations);
    });
  }

  fn undeclare_leds(&self, identifications: impl Into<LedIdentifications>) {
    with_led_system(self, |led_system| {
      led_system.undeclare(self.current_system_id(), identifications);
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

  fn set_led_conflict_resolution(
    &self,
    identifications: impl Into<LedIdentifications>,
    resolution: LedConflictResolution,
  ) {
    with_led_system(self, |led_system| {
      led_system.set_conflict_resolution(identifications, resolution);
    });
  }
}

fn with_led_system<T>(ctx: &Context, f: impl FnOnce(&mut LedSystem) -> T) {
  if let Some(mut led_system) = ctx.systems.get::<LedSystem>() {
    f(&mut led_system);
  } else {
    log::error!("LedSystem not running.");
  }
}
