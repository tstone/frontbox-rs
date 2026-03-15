use crate::prelude::*;

const WATCHDOG_TIMER_NAME: &'static str = "watchdog";

pub struct Watchdog;

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
    // Neuron seems to expect the watchdog to always be running (e.g. otherwise the low voltage drivers don't work)
    // Once the smart power filter board firmware is updated, there will likely be a separate command to enable/disable high voltage
    // For now just always start it up
    Watchdog::enable(ctx);

    // Enable watchdog command
    ctx.register_command::<EnableWatchdog>(move |_, ctx| {
      Watchdog::enable(ctx);
    });

    // Disable watchdog command
    ctx.register_command::<DisableWatchdog>(move |_, ctx| {
      Watchdog::disable(ctx);
    });
  }

  fn on_timer(&mut self, timer_name: &'static str, ctx: &mut Context) {
    if timer_name == WATCHDOG_TIMER_NAME {
      ctx.command(WatchdogPing);
    }
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {
    if event.is::<Shutdown>() {
      Watchdog::disable(ctx);
    }
  }
}

pub struct EnableWatchdog;
pub struct DisableWatchdog;
