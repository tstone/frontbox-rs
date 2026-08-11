use crate::prelude::*;

/// A system which continually pings the FAST hardware to keep 48v active. This is required to use drivers.
pub struct WatchdogSystem {
  cue_handle: Option<u64>,
}

impl WatchdogSystem {
  pub fn new() -> Self {
    Self { cue_handle: None }
  }
}

impl WatchdogSystem {
  pub fn enable(ctx: &SystemContext) {
    log::info!("Enabling watchdog with {:?}", ctx.watchdog_interval);

    ctx.cue(WatchdogPing, Cue::Loop(ctx.watchdog_interval));
    ctx.expect::<Machine>().ping_watchdog();
  }

  pub fn disable(&self, ctx: &SystemContext) {
    log::info!("Disabling watchdog");
    ctx.expect::<Machine>().clear_watchdog();

    if let Some(handle) = &self.cue_handle {
      ctx.cancel_cue(*handle);
    }
  }
}

impl System for WatchdogSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    // Neuron seems to expect the watchdog to always be running (e.g. otherwise the low voltage drivers don't work)
    // Once the smart power filter board firmware is updated, there will likely be a separate command to enable/disable high voltage
    // For now just always start it up
    WatchdogSystem::enable(ctx);
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<WatchdogPing>() {
      log::trace!("Watchdog event => Ping");
      ctx.expect::<Machine>().ping_watchdog();
    }
  }

  fn on_despawn(&mut self, ctx: &SystemContext) {
    self.disable(ctx);
  }
}

#[derive(serde::Serialize, Event)]
struct WatchdogPing;
