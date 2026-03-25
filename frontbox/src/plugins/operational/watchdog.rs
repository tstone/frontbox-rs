use crate::prelude::*;

/// A system which continually pings the FAST hardware to keep 48v active. This is required to use drivers.
pub struct Watchdog {
  cue_handle: Option<u64>,
}

impl Watchdog {
  pub fn new() -> Self {
    Self { cue_handle: None }
  }
}

impl Watchdog {
  pub fn enable(ctx: &mut Context, systems: &Systems) {
    let app_config = ctx.expect::<AppConfig>();
    log::info!("Enabling watchdog with {:?}", app_config.watchdog_tick);

    ctx.cue(WatchdogPing, Cue::Loop(app_config.watchdog_tick));
    systems.expect::<Machine>().ping_watchdog();
  }

  pub fn disable(&self, ctx: &mut Context, systems: &Systems) {
    log::info!("Disabling watchdog");
    systems.expect::<Machine>().clear_watchdog();

    if let Some(handle) = &self.cue_handle {
      ctx.cancel_cue(*handle);
    }
  }
}

impl System for Watchdog {
  fn on_startup(&mut self, ctx: &mut Context, systems: &Systems) {
    // Neuron seems to expect the watchdog to always be running (e.g. otherwise the low voltage drivers don't work)
    // Once the smart power filter board firmware is updated, there will likely be a separate command to enable/disable high voltage
    // For now just always start it up
    Watchdog::enable(ctx, systems);
  }

  fn on_cue(&mut self, _cue: &dyn Signal, _ctx: &mut Context, systems: &Systems) {
    log::trace!("Watchdog cue => Ping");
    systems.expect::<Machine>().ping_watchdog();
  }

  fn on_shutdown(&mut self, ctx: &mut Context, systems: &Systems) {
    self.disable(ctx, systems);
  }
}

pub struct EnableWatchdog;
pub struct DisableWatchdog;
