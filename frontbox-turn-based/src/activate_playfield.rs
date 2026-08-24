use std::collections::HashMap;

use crate::{PlayerTurnBeginning, PlayerTurnEnding};
use frontbox::prelude::*;

/// Turn drivers on and off in bulk, automatically (manual supported too)
/// This is mainly used to call activate driver/automatic for all playfield drivers at the start and stop of a turn
#[derive(Clone, Debug, Default)]
pub struct ActivatePlayfieldSystem {
  // driver name, switch name
  driver_table: HashMap<&'static str, &'static str>,
}

impl ActivatePlayfieldSystem {
  pub fn new() -> Self {
    Self {
      driver_table: HashMap::new(),
    }
  }

  pub fn driver(
    mut self,
    driver_name: impl Into<&'static str>,
    switch_name: impl Into<&'static str>,
  ) -> Self {
    self
      .driver_table
      .insert(driver_name.into(), switch_name.into());
    self
  }

  fn activate(&self, ctx: &SystemContext) {
    let machine = ctx.expect::<Machine>();

    for (driver, switch) in &self.driver_table {
      machine.activate_driver(driver, ActivationMode::Automatic(switch), ctx.into());
    }
  }

  fn deactivate(&self, ctx: &SystemContext) {
    for driver in self.driver_table.keys() {
      ctx.deactivate_driver(driver, DeactivationMode::Disabled);
    }
  }
}

impl System for ActivatePlayfieldSystem {
  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<PlayerTurnBeginning>() {
      log::info!("Activating playfield drivers due to turn start");
      self.activate(ctx);
      ctx.emit(ActivatedPlayfieldDrivers);
    } else if event.is::<PlayerTurnEnding>() {
      log::info!("Deactivating playfield drivers due to turn end");
      self.deactivate(ctx);
    }
  }
}

#[derive(serde::Serialize, Event)]
pub struct ActivatedPlayfieldDrivers;
