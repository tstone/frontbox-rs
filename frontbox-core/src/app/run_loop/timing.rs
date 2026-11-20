use super::*;

impl RunLoop {
  pub(super) fn tick(&mut self, delta: Duration) {
    delivering("tick", || {
      // Tick first is where most systems should do any time-based processing
      apply_to_systems(
        &self.groups,
        &self.base,
        &self.app_sender,
        &self.tracers,
        |system, ctx| {
          system.on_tick(delta, ctx);
        },
      );
      self.routines.tick(
        delta,
        &self.groups,
        &self.base,
        &self.app_sender,
        &self.tracers,
      );
      // Render is when systems that depend on what others systems have done (e.g. LED or DMD rendering) occur
      apply_to_systems(
        &self.groups,
        &self.base,
        &self.app_sender,
        &self.tracers,
        |system, ctx| {
          system.on_render(ctx);
        },
      );
    });
  }

  pub(super) fn create_timer(
    &self,
    handle: SystemHandle,
    timer_id: u64,
    timer: Timer,
    signals: Vec<Box<dyn Event>>,
  ) {
    if let Some(mut system) = self.groups.system(handle) {
      log::debug!(target: "frontbox::timers", "Creating timer for {} ({})", system.name(), handle.id);
      system.create_timer(timer, timer_id, signals);
    } else {
      log::warn!(
        target: "frontbox::timers",
        "No system found with ID {}, cannot create timer {:?}",
        handle.id,
        timer
      );
    }
  }

  pub(super) fn cancel_timer(&self, handle: SystemHandle, timer_id: u64) {
    if let Some(mut system) = self.groups.system(handle) {
      system.cancel_timer(timer_id);
    } else {
      log::warn!(
        target: "frontbox::timers",
        "No system found with ID {}, cannot cancel timer with ID {}",
        handle.id,
        timer_id
      );
    }
  }

  pub(super) fn set_timer_paused(&self, handle: SystemHandle, timer_id: u64, paused: bool) {
    if let Some(mut system) = self.groups.system(handle) {
      log::debug!(
        target: "frontbox::timers",
        "{} timer {} for {} ({})",
        if paused { "Pausing" } else { "Resuming" },
        timer_id,
        system.name(),
        handle.id
      );
      system.set_timer_paused(timer_id, paused);
    } else {
      log::warn!(
        target: "frontbox::timers",
        "No system found with ID {}, cannot pause or resume timer with ID {}",
        handle.id,
        timer_id
      );
    }
  }
}

/// If run loop processing starts running behind, this is a multiplier for how many intervals to allow before clamping the delta passed to systems.
/// This is to avoid things incorrectly appearing as happening at once after a stall in the run loop processing. ie. it's the most that downstream
/// systems will be told that time has passed since the last tick, even if more time has actually passed.
const MAX_TICK_INTERVALS: u32 = 4;

/// Calculate actual passed wall clock time, clamped to `MAX_TICK_INTERVALS`
pub(super) fn tick_delta(elapsed: Duration, interval: Duration) -> Duration {
  elapsed.min(interval * MAX_TICK_INTERVALS)
}

pub(super) fn spawn_system_ticker(tick: Duration, sender: watch::Sender<()>, resync: Arc<Notify>) {
  let mut timer_interval = tokio::time::interval(tick);
  timer_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

  tokio::spawn(async move {
    loop {
      tokio::select! {
          _ = timer_interval.tick() => {
              let _ = sender.send(());
          }
          _ = resync.notified() => {
              timer_interval.reset_immediately();
          }
      }
    }
  });
}

#[cfg(test)]
mod tests {
  use std::cell::Cell;
  use std::rc::Rc;

  use super::*;
  use crate::app::run_loop::test_support::*;

  fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
  }

  #[test]
  fn every_system_ticks_before_any_renders() {
    let mut h = Harness::new();
    let log = Log::default();
    h.spawn(Probe::<1>(log.clone()));
    h.spawn(Probe::<2>(log.clone()));
    log.take();

    h.tick_by(ms(10));

    assert_eq!(
      log.take(),
      ["1 on_tick", "2 on_tick", "1 on_render", "2 on_render"]
    );
  }

  /// Timers a `Poke` to itself 100ms after it spawns, and counts the `Poke`s it hears
  struct Timed {
    timer: Rc<Cell<u64>>,
    pokes: Rc<Cell<u32>>,
  }

  impl System for Timed {
    fn on_spawn(&mut self, ctx: &SystemContext) {
      self.timer.set(ctx.timers.start(Poke, Timer::Once(ms(100))));
    }

    fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
      if event.is::<Poke>() {
        self.pokes.set(self.pokes.get() + 1);
      }
    }
  }

  fn spawn_timed(h: &mut Harness) -> (SystemHandle, u64, Rc<Cell<u32>>) {
    let (timer, pokes) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let handle = h.spawn(Timed {
      timer: timer.clone(),
      pokes: pokes.clone(),
    });
    (handle, timer.get(), pokes)
  }

  #[test]
  fn a_timer_signals_its_own_system_once_its_time_has_passed() {
    let mut h = Harness::new();
    let log = Log::default();
    h.spawn(Probe::<1>(log.clone()));
    let (_, _, pokes) = spawn_timed(&mut h);

    h.tick_by(ms(60));
    assert_eq!(pokes.get(), 0);

    h.tick_by(ms(60));
    assert_eq!(pokes.get(), 1);

    h.tick_by(ms(200));
    assert_eq!(pokes.get(), 1, "a `Timer::Once` fires once");
    assert!(
      !log.take().contains(&"1 heard Poke".to_string()),
      "only the system that started it hears it"
    );
  }

  #[test]
  fn a_cancelled_timer_never_signals() {
    let mut h = Harness::new();
    let (handle, timer, pokes) = spawn_timed(&mut h);

    h.send(AppMessage::CancelTimer(handle, timer));
    h.settle();
    h.tick_by(ms(200));

    assert_eq!(pokes.get(), 0);
  }

  #[test]
  fn a_paused_timer_signals_only_after_it_is_resumed() {
    let mut h = Harness::new();
    let (handle, timer, pokes) = spawn_timed(&mut h);

    h.tick_by(ms(60));
    h.send(AppMessage::SetTimerPaused(handle, timer, true));
    h.settle();
    h.tick_by(ms(200));
    assert_eq!(pokes.get(), 0, "time doesn't count toward a paused timer");

    h.send(AppMessage::SetTimerPaused(handle, timer, false));
    h.settle();
    h.tick_by(ms(30));
    assert_eq!(pokes.get(), 0, "it picks up at 60ms of 100ms");

    h.tick_by(ms(10));
    assert_eq!(pokes.get(), 1);
  }

  #[test]
  fn tick_delta_is_the_measured_time_clamped_to_a_few_intervals() {
    let interval = Duration::from_millis(80);
    assert_eq!(
      tick_delta(Duration::from_millis(95), interval),
      Duration::from_millis(95)
    );
    assert_eq!(
      tick_delta(Duration::from_millis(40), interval),
      Duration::from_millis(40)
    );
    assert_eq!(
      tick_delta(Duration::from_secs(3), interval),
      interval * MAX_TICK_INTERVALS
    );
  }
}
