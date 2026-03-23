use crate::prelude::*;

/// A system which continually pings the FAST hardware to keep 48v active. This is required to use drivers.
pub struct Watchdog {
  cue_handle: Option<u64>,
}

impl Watchdog {
  pub fn new() -> Box<Self> {
    Box::new(Watchdog { cue_handle: None })
  }
}

impl Watchdog {
  fn enable(ctx: &mut Context) {
    let app_config = ctx.expect::<AppConfig>();

    ctx.cue(WatchdogPing, Cue::Loop(app_config.watchdog_tick));
    ctx.command(WatchdogPing);
  }

  fn disable(&self, ctx: &mut Context) {
    ctx.command(ClearWatchdog);
    if let Some(handle) = &self.cue_handle {
      ctx.cancel_cue(*handle);
    }
  }
}

impl System for Watchdog {
  fn on_startup(&mut self, ctx: &mut Context) {
    ctx.register_command::<EnableWatchdog>();
    ctx.register_command::<DisableWatchdog>();

    // Neuron seems to expect the watchdog to always be running (e.g. otherwise the low voltage drivers don't work)
    // Once the smart power filter board firmware is updated, there will likely be a separate command to enable/disable high voltage
    // For now just always start it up
    Watchdog::enable(ctx);
  }

  fn on_command(&mut self, command: &dyn Signal, ctx: &mut Context) {
    if let Some(_) = command.downcast_ref::<EnableWatchdog>() {
      Watchdog::enable(ctx);
    } else if let Some(_) = command.downcast_ref::<DisableWatchdog>() {
      self.disable(ctx);
    }
  }

  fn on_cue(&mut self, _cue: &dyn Signal, ctx: &mut Context) {
    ctx.command(WatchdogPing);
  }

  fn on_shutdown(&mut self, ctx: &mut Context) {
    self.disable(ctx);
  }
}

pub struct EnableWatchdog;
pub struct DisableWatchdog;
