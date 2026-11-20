use std::cell::RefMut;

use frontbox_core::prelude::*;
use frontbox_gfx::prelude::*;

use crate::*;

/// Declaring through light management from a system. Each method resolves its targets, typically an `LightQ`, to the
/// lights they reach, and declares at `z` as the calling system.
pub trait LightManagerExt {
  /// The running light management, for calls not covered below. `None` if it isn't running, or is in use.
  fn light_manager(&self) -> Option<RefMut<'_, dyn LightManager>>;

  /// Declare one color at `z` for every light the targets resolve to
  fn declare_lights<T: Contextual<Vec<LightAddress>>>(&self, targets: &T, z: i8, color: Rgba<u8>);
  /// Declare one color at `z` for every light the targets resolve to, flashing on and off
  fn declare_lights_flashing<T: Contextual<Vec<LightAddress>>>(&self, targets: &T, z: i8, color: Rgba<u8>);
  /// Declare colors at `z` for the targets, given by `colors` from the number of lights they resolve to
  fn declare_light_colors<T: Contextual<Vec<LightAddress>>>(
    &self,
    targets: &T,
    z: i8,
    colors: impl FnOnce(usize) -> Vec<Rgba<u8>>,
  );
  /// Declare one color at `z` for every light the targets resolve to, not shown until
  /// [`activate_light_declarations`](Self::activate_light_declarations)
  fn declare_lights_inactive<T: Contextual<Vec<LightAddress>>>(&self, targets: &T, z: i8, color: Rgba<u8>);
  /// Take back what this system declared at `z` on the lights the targets resolve to
  fn undeclare_lights<T: Contextual<Vec<LightAddress>>>(&self, targets: &T, z: i8);
  fn activate_light_declarations(&self);
  fn deactivate_light_declarations(&self);
  fn set_light_conflict_resolution<T: Contextual<Vec<LightAddress>>>(&self, targets: &T, resolution: LightConflictResolution);
}

impl<'a> LightManagerExt for SystemContext<'a> {
  fn light_manager(&self) -> Option<RefMut<'_, dyn LightManager>> {
    self.systems.get_dyn::<dyn LightManager>()
  }

  fn declare_lights<T: Contextual<Vec<LightAddress>>>(&self, targets: &T, z: i8, color: Rgba<u8>) {
    self.declare_light_colors(targets, z, |count| vec![color; count]);
  }

  fn declare_lights_flashing<T: Contextual<Vec<LightAddress>>>(&self, targets: &T, z: i8, color: Rgba<u8>) {
    with_light_manager(self, |manager| {
      let colors = one_color(targets.resolve(self), color);
      manager.declare(self.id(), z, &colors, LightMode::Flashing);
    });
  }

  fn declare_light_colors<T: Contextual<Vec<LightAddress>>>(
    &self,
    targets: &T,
    z: i8,
    colors: impl FnOnce(usize) -> Vec<Rgba<u8>>,
  ) {
    with_light_manager(self, |manager| {
      let lights = targets.resolve(self);
      let colors = colors(lights.len());
      let colors: Vec<_> = lights.into_iter().zip(colors).collect();
      manager.declare(self.id(), z, &colors, LightMode::Solid);
    });
  }

  fn declare_lights_inactive<T: Contextual<Vec<LightAddress>>>(&self, targets: &T, z: i8, color: Rgba<u8>) {
    with_light_manager(self, |manager| {
      let colors = one_color(targets.resolve(self), color);
      manager.declare_inactive(self.id(), z, &colors, LightMode::Solid);
    });
  }

  fn undeclare_lights<T: Contextual<Vec<LightAddress>>>(&self, targets: &T, z: i8) {
    with_light_manager(self, |manager| {
      manager.undeclare(self.id(), z, &targets.resolve(self));
    });
  }

  fn deactivate_light_declarations(&self) {
    with_light_manager(self, |manager| {
      manager.deactivate_by_system(self.id());
    });
  }

  fn activate_light_declarations(&self) {
    with_light_manager(self, |manager| {
      manager.activate_by_system(self.id());
    });
  }

  fn set_light_conflict_resolution<T: Contextual<Vec<LightAddress>>>(&self, targets: &T, resolution: LightConflictResolution) {
    with_light_manager(self, |manager| {
      manager.set_conflict_resolution(&targets.resolve(self), resolution);
    });
  }
}

fn one_color(lights: Vec<LightAddress>, color: Rgba<u8>) -> Vec<(LightAddress, Rgba<u8>)> {
  lights.into_iter().map(|light| (light, color)).collect()
}

fn with_light_manager<T>(ctx: &SystemContext, f: impl FnOnce(&mut dyn LightManager) -> T) {
  if let Some(mut manager) = ctx.light_manager() {
    f(&mut *manager);
  } else {
    log::error!(target: "frontbox::lighting", "Light management is not running.");
  }
}
