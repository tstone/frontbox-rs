use super::*;

impl RunLoop {
  pub(super) fn tick(&self, delta: Duration) {
    // Tick first is where most systems should do any time-based processing
    apply_to_systems(&self.groups, &self.base, &self.app_sender, &self.tracers, |system, ctx| {
      system.on_tick(delta, ctx);
    });
    // Render is when systems that depend on what others systems have done (e.g. LED or DMD rendering) occur
    apply_to_systems(&self.groups, &self.base, &self.app_sender, &self.tracers, |system, ctx| {
      system.on_render(ctx);
    });
  }

  pub(super) fn create_cue(&self, handle: SystemHandle, cue_id: u64, cue: Cue, signals: Vec<Box<dyn Event>>) {
    if let Some(mut system) = self.groups.system(handle) {
      log::debug!("Creating cue for {} ({})", system.name(), handle.id);
      system.create_cue(cue, cue_id, signals);
    } else {
      log::warn!(
        "Cueing: No system found with ID {}, cannot create cue {:?}",
        handle.id,
        cue
      );
    }
  }

  pub(super) fn cancel_cue(&self, handle: SystemHandle, cue_id: u64) {
    if let Some(mut system) = self.groups.system(handle) {
      system.cancel_cue(cue_id);
    } else {
      log::warn!(
        "No system found with ID {}, cannot cancel cue with ID {}",
        handle.id,
        cue_id
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
  use super::*;

  #[test]
  fn tick_delta_is_the_measured_time_clamped_to_a_few_intervals() {
    let interval = Duration::from_millis(80);
    assert_eq!(tick_delta(Duration::from_millis(95), interval), Duration::from_millis(95));
    assert_eq!(tick_delta(Duration::from_millis(40), interval), Duration::from_millis(40));
    assert_eq!(tick_delta(Duration::from_secs(3), interval), interval * MAX_TICK_INTERVALS);
  }
}
