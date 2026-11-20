use frontbox_core::prelude::*;

use crate::{GameEnding, TurnEnding, TurnStarting};

/// Makes the playfield live for each turn. Drivers matching `drivers` that fire from a switch (slings, pops, flippers,
/// etc.) are activated when a turn begins (`TurnStarting`), and disabled when it ends (`TurnEnding`, including on a
/// tilt) or the game ends (`GameEnding`). Drivers fired by the game instead (virtual switch triggers like ejects and
/// resets) are left alone even when they match.
///
/// It reads only routine events, so it works with any `GameManager`. Add it at the root.
///
/// ```rust,ignore
/// app.system(PlayfieldDrivers::new(DriverQ::tag::<Playfield>()))
/// ```
pub struct PlayfieldDrivers {
  drivers: DriverQ,
}

impl PlayfieldDrivers {
  pub fn new(drivers: DriverQ) -> Self {
    Self { drivers }
  }

  fn switch_triggered(&self, ctx: &SystemContext) -> Vec<&'static str> {
    self
      .drivers
      .query_names(ctx.into())
      .into_iter()
      .filter(|driver| ctx.drivers.is_switch_triggered(driver))
      .collect()
  }

  fn activate(&self, ctx: &SystemContext) {
    log::info!(target: "frontbox::playfield_drivers", "Activating playfield drivers for the turn");
    for driver in self.switch_triggered(ctx) {
      ctx.drivers.activate(driver, ActivationMode::Automatic);
    }
  }

  fn deactivate(&self, ctx: &SystemContext) {
    log::info!(target: "frontbox::playfield_drivers", "Deactivating playfield drivers");
    for driver in self.switch_triggered(ctx) {
      ctx.drivers.deactivate(driver, DeactivationMode::Disabled);
    }
  }
}

impl System for PlayfieldDrivers {
  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<RoutineStarted<TurnStarting>>() {
      self.activate(ctx);
    } else if event.is::<RoutineStarted<TurnEnding>>() || event.is::<RoutineStarted<GameEnding>>() {
      self.deactivate(ctx);
    }
  }
}
