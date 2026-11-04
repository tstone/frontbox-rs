use frontbox::prelude::*;

use crate::TiltSystem;

pub trait TiltExt {
  /// Give the current player another tilt warning. See `TiltSystem::grant_warning`. Called from a system in the player
  /// template, as that's where the player's `TiltSystem` is found.
  fn grant_tilt_warning(&self);
  /// Tilt warnings the current player has left
  fn tilt_warnings(&self) -> u8;
}

impl<'a> TiltExt for SystemContext<'a> {
  fn grant_tilt_warning(&self) {
    if let Some(mut tilt) = self.systems.get::<TiltSystem>() {
      tilt.grant_warning(self.into());
    } else {
      log::warn!(target: "frontbox::tilt", "TiltSystem not running.");
    }
  }

  fn tilt_warnings(&self) -> u8 {
    self
      .systems.get::<TiltSystem>()
      .map(|tilt| tilt.warnings())
      .unwrap_or(0)
  }
}
