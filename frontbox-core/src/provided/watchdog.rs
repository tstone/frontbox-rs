use crate::machine::*;
use crate::prelude::*;

/// A renewal asked for this much later than its period is logged
const LATE_WARNING: Duration = Duration::from_millis(100);

/// A system which continually pings the FAST hardware to keep 48v active. This is required to use drivers.
pub struct WatchdogSystem {
  handle: SystemHandle,
  timer_handle: Option<u64>,
  /// How often the run loop asks the machine to renew the watchdog
  period: Duration,
  /// When it last asked, to report one that was late. None while the watchdog is off.
  last_ping: Option<std::time::Instant>,
}

impl WatchdogSystem {
  pub fn new() -> Self {
    Self {
      timer_handle: None,
      period: Duration::ZERO,
      last_ping: None,
      handle: SystemHandle::default(),
    }
  }
}

impl WatchdogSystem {
  pub fn enable(&mut self, ctx: &ServiceContext) {
    let ctx = ctx.for_system(self.handle);
    log::info!(target: "frontbox::watchdog", "🐶 Enabling watchdog with {:?}", ctx.watchdog_interval);

    // renew it slightly ahead of the configured interval to add some buffer for any latency
    self.period =
      ctx.watchdog_interval.unwrap_or(Duration::from_millis(1500)) - Duration::from_millis(250);
    self.timer_handle = Some(ctx.timers.start(WatchdogPing, Timer::Forever(self.period)));
    self.ping(&ctx);
  }

  /// Ask the machine to renew the watchdog, reporting if the run loop was late to. The machine logs a renewal that came
  /// close to the watchdog running out; this says whether the run loop is why.
  fn ping(&mut self, ctx: &SystemContext) {
    let now = std::time::Instant::now();
    if let Some(last) = self.last_ping {
      let late = (now - last).saturating_sub(self.period);
      if late >= LATE_WARNING {
        log::warn!(target: "frontbox::watchdog", "🐶 The run loop asked for a watchdog renewal {:?} late", late);
      }
    }
    self.last_ping = Some(now);
    ctx.systems.expect::<MachineSystem>().ping_watchdog();
  }

  pub fn disable(&mut self, ctx: &ServiceContext) {
    let ctx = ctx.for_system(self.handle);

    log::info!(target: "frontbox::watchdog", "🐶 Disabling watchdog");
    ctx.systems.expect::<MachineSystem>().clear_watchdog();

    ctx.timers.cancel(self.timer_handle.take());
    self.last_ping = None;
  }
}

impl System for WatchdogSystem {
  fn description() -> Option<&'static str> {
    Some("Pings the FAST hardware so it keeps 48V on. Without it, no coils can fire.")
  }

  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = ctx.handle();

    // Neuron seems to expect the watchdog to always be running (e.g. otherwise the low voltage drivers don't work)
    // Once the smart power filter board firmware is updated, there will likely be a separate command to enable/disable high voltage
    // For now just always start it up
    self.enable(ctx.into());
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<WatchdogPing>() {
      log::trace!(target: "frontbox::watchdog", "🐶 Watchdog event => Ping");
      self.ping(ctx);
    } else if event.is::<MachineReconnected>() && self.timer_handle.is_some() {
      // feed it now rather than at the next ping, as the hardware's has been running down since it was brought up
      self.last_ping = None;
      self.ping(ctx);
    }
  }

  fn on_despawn(&mut self, ctx: &SystemContext) {
    self.disable(ctx.into());
  }
}

#[derive(serde::Serialize, Event)]
struct WatchdogPing;
