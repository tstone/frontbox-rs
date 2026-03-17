use crate::prelude::*;

const WATCHDOG_TIMER_NAME: &'static str = "watchdog";

/// A system which continually pings the FAST hardware to keep 48v active. This is required to use drivers.
pub struct Watchdog;

impl Watchdog {
  pub fn new() -> Box<Self> {
    Box::new(Watchdog)
  }
}

impl Watchdog {
  fn enable(ctx: &mut Context) {
    let app_config = ctx.expect::<AppConfig>();

    ctx.set_timer(
      WATCHDOG_TIMER_NAME,
      app_config.watchdog_tick,
      TimerMode::Repeating,
    );
    ctx.command(WatchdogPing);
  }

  fn disable(ctx: &mut Context) {
    ctx.command(ClearWatchdog);
    ctx.clear_timer(WATCHDOG_TIMER_NAME);
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

  fn on_command(&mut self, command: &dyn Command, ctx: &mut Context) {
    if let Some(_) = command.downcast_ref::<EnableWatchdog>() {
      Watchdog::enable(ctx);
    } else if let Some(_) = command.downcast_ref::<DisableWatchdog>() {
      Watchdog::disable(ctx);
    }
  }

  fn on_timer(&mut self, timer_name: &'static str, ctx: &mut Context) {
    ctx.command(WatchdogPing);
  }

  fn on_shutdown(&mut self, ctx: &mut Context) {
    Watchdog::disable(ctx);
  }
}

pub struct EnableWatchdog;
pub struct DisableWatchdog;
