use crate::machine::*;
use crate::prelude::*;

/// A system which continually pings the FAST hardware to keep 48v active. This is required to use drivers.
pub struct WatchdogSystem {
  handle: SystemHandle,
  cue_handle: Option<u64>,
}

impl WatchdogSystem {
  pub fn new() -> Self {
    Self {
      cue_handle: None,
      handle: SystemHandle::default(),
    }
  }
}

impl WatchdogSystem {
  pub fn enable(&mut self, ctx: &ServiceContext) {
    let ctx = ctx.for_system(self.handle);
    log::info!(target: "frontbox_core::watchdog", "🐶 Enabling watchdog with {:?}", ctx.watchdog_interval);

    // renew watchdog slightly before it expires
    self.cue_handle = Some(ctx.cues.start(
      WatchdogPing,
      Cue::Forever(
        // renew it slightly ahead of the configured interval to add some buffer for any latency
        ctx.watchdog_interval.unwrap_or(Duration::from_millis(1500)) - Duration::from_millis(250),
      ),
    ));
    ctx.systems.expect::<MachineSystem>().ping_watchdog();
  }

  pub fn disable(&mut self, ctx: &ServiceContext) {
    let ctx = ctx.for_system(self.handle);

    log::info!(target: "frontbox_core::watchdog", "🐶 Disabling watchdog");
    ctx.systems.expect::<MachineSystem>().clear_watchdog();

    ctx.cues.cancel(self.cue_handle.take());
  }
}

impl System for WatchdogSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = ctx.handle();

    // Neuron seems to expect the watchdog to always be running (e.g. otherwise the low voltage drivers don't work)
    // Once the smart power filter board firmware is updated, there will likely be a separate command to enable/disable high voltage
    // For now just always start it up
    self.enable(ctx.into());
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<WatchdogPing>() {
      log::trace!(target: "frontbox_core::watchdog", "🐶 Watchdog event => Ping");
      ctx.systems.expect::<MachineSystem>().ping_watchdog();
    } else if event.is::<MachineReconnected>() && self.cue_handle.is_some() {
      // feed it now rather than at the next ping, as the hardware's has been running down since it was brought up
      ctx.systems.expect::<MachineSystem>().ping_watchdog();
    }
  }

  fn on_despawn(&mut self, ctx: &SystemContext) {
    self.disable(ctx.into());
  }
}

#[derive(serde::Serialize, Event)]
struct WatchdogPing;
