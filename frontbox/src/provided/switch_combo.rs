use crate::prelude::*;

/// Runs a closure when every switch matched by a query is closed at the same time.
///
/// By default the combo completes the moment the last switch closes. Two optional constraints narrow that:
/// - `within`: every switch must close within the given window of the first one (e.g. a double flip)
/// - `held_for`: once complete, every switch must stay closed for the given duration (e.g. hold to quit)
///
/// The combo fires at most once per completion: releasing any of the switches re-arms it, and when `within` is set
/// the window only restarts once every switch has been released.
///
/// ```ignore
/// SwitchComboSystem::new(SwitchQ::names(["l_flipper", "start"]))
///   .held_for(Duration::from_millis(1250))
///   .on_combo(|ctx| ctx.emit(Example));
/// ```
pub struct SwitchComboSystem {
  query: SwitchQ,
  // query is resolved into fixed list of names on spawn
  switch_names: Vec<&'static str>,
  within: Option<Duration>,
  held_for: Option<Duration>,
  on_combo: Box<dyn FnMut(&SystemContext) + Send + Sync>,
  window_started: bool,
  window_cue: Option<u64>,
  hold_cue: Option<u64>,
  fired: bool,
}

impl SwitchComboSystem {
  pub fn new(switches: impl Into<SwitchQ>) -> Self {
    Self {
      query: switches.into(),
      switch_names: Vec::new(),
      within: None,
      held_for: None,
      on_combo: Box::new(|_| {}),
      window_started: false,
      window_cue: None,
      hold_cue: None,
      fired: false,
    }
  }

  /// Every switch must close within `window` of the first one
  pub fn within(mut self, window: Duration) -> Self {
    self.within = Some(window);
    self
  }

  /// Every switch must stay closed for `duration` before the combo fires
  pub fn held_for(mut self, duration: Duration) -> Self {
    self.held_for = Some(duration);
    self
  }

  /// Runs when the combo completes
  pub fn on_combo(mut self, on_combo: impl FnMut(&SystemContext) + Send + Sync + 'static) -> Self {
    self.on_combo = Box::new(on_combo);
    self
  }

  fn is_member(&self, switch: &Switch) -> bool {
    self.switch_names.contains(&switch.name)
  }

  fn closed_count(&self, ctx: &SystemContext) -> usize {
    self
      .switch_names
      .iter()
      .filter(|name| ctx.switches.is_closed(name).unwrap_or(false))
      .count()
  }

  fn on_member_closed(&mut self, ctx: &SystemContext) {
    if let Some(window) = self.within {
      if !self.window_started {
        self.window_started = true;
        self.window_cue = Some(ctx.cue(WindowElapsed, window.once()));
      }
    }

    if self.fired || self.hold_cue.is_some() || self.closed_count(ctx) < self.switch_names.len() {
      return;
    }

    // Too slow: the window ran out before the last switch closed
    if self.within.is_some() && self.window_cue.is_none() {
      return;
    }

    match self.held_for {
      Some(duration) => self.hold_cue = Some(ctx.cue(Held, duration.once())),
      None => self.fire(ctx),
    }
  }

  fn on_member_opened(&mut self, ctx: &SystemContext) {
    self.fired = false;
    if let Some(cue_id) = self.hold_cue.take() {
      ctx.cancel_cue(cue_id);
    }

    if self.closed_count(ctx) == 0 {
      self.window_started = false;
      if let Some(cue_id) = self.window_cue.take() {
        ctx.cancel_cue(cue_id);
      }
    }
  }

  fn on_held(&mut self, ctx: &SystemContext) {
    self.hold_cue = None;
    if self.closed_count(ctx) == self.switch_names.len() {
      self.fire(ctx);
    }
  }

  fn fire(&mut self, ctx: &SystemContext) {
    self.fired = true;
    (self.on_combo)(ctx);
  }
}

impl System for SwitchComboSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.switch_names = self.query.query_names(ctx.into());
    if self.switch_names.is_empty() {
      log::warn!(target: "frontbox::switch_combo", "Switch combo query {:?} matched no switches; it will never fire", self.query);
    }
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(event) = event.downcast_ref::<SwitchClosed>() {
      if self.is_member(&event.switch) {
        self.on_member_closed(ctx);
      }
    } else if let Some(event) = event.downcast_ref::<SwitchOpened>() {
      if self.is_member(&event.switch) {
        self.on_member_opened(ctx);
      }
    } else if event.is::<Held>() {
      self.on_held(ctx);
    } else if event.is::<WindowElapsed>() {
      self.window_cue = None;
    }
  }
}

#[derive(serde::Serialize, Event)]
struct Held;

#[derive(serde::Serialize, Event)]
struct WindowElapsed;

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::Arc;
  use std::sync::atomic::{AtomicUsize, Ordering};

  const A: &str = "a";
  const B: &str = "b";

  struct Fixture {
    combo: SwitchComboSystem,
    ctx: TestContext,
    fired: Arc<AtomicUsize>,
    states: Vec<SwitchState>,
  }

  impl Fixture {
    fn spawn(configure: impl FnOnce(SwitchComboSystem) -> SwitchComboSystem) -> Self {
      let mut ctx = TestContext::default();
      for (id, name) in [A, B].into_iter().enumerate() {
        ctx.insert_switch(Switch {
          name,
          id,
          ..Default::default()
        });
      }
      let states = vec![SwitchState::Open, SwitchState::Open];
      ctx.update_switch_states(states.clone());

      let fired = Arc::new(AtomicUsize::new(0));
      let counter = fired.clone();
      let mut combo =
        configure(SwitchComboSystem::new(SwitchQ::names([A, B]))).on_combo(move |_| {
          counter.fetch_add(1, Ordering::SeqCst);
        });
      combo.on_spawn(&ctx.sys_ctx());
      Self {
        combo,
        ctx,
        fired,
        states,
      }
    }

    fn set(&mut self, name: &'static str, closed: bool) {
      let switch = self.ctx.base.switches.by_name(name).unwrap().clone();
      self.states[switch.id] = if closed {
        SwitchState::Closed
      } else {
        SwitchState::Open
      };
      self.ctx.update_switch_states(self.states.clone());
      if closed {
        self
          .combo
          .on_event(&SwitchClosed::new(switch), &self.ctx.sys_ctx());
      } else {
        self
          .combo
          .on_event(&SwitchOpened::new(switch), &self.ctx.sys_ctx());
      }
    }

    fn deliver(&mut self, event: impl Event) {
      self.combo.on_event(&event, &self.ctx.sys_ctx());
    }

    fn fired(&self) -> usize {
      self.fired.load(Ordering::SeqCst)
    }
  }

  #[test]
  fn fires_when_all_closed() {
    let mut f = Fixture::spawn(|c| c);
    f.set(A, true);
    assert_eq!(f.fired(), 0);
    f.set(B, true);
    assert_eq!(f.fired(), 1);
  }

  #[test]
  fn fires_once_until_released() {
    let mut f = Fixture::spawn(|c| c);
    f.set(A, true);
    f.set(B, true);
    f.set(B, true);
    assert_eq!(f.fired(), 1);
    f.set(B, false);
    f.set(B, true);
    assert_eq!(f.fired(), 2);
  }

  #[test]
  fn within_window_fires() {
    let mut f = Fixture::spawn(|c| c.within(Duration::from_millis(250)));
    f.set(A, true);
    f.set(B, true);
    assert_eq!(f.fired(), 1);
  }

  #[test]
  fn outside_window_does_not_fire() {
    let mut f = Fixture::spawn(|c| c.within(Duration::from_millis(250)));
    f.set(A, true);
    f.deliver(WindowElapsed);
    f.set(B, true);
    assert_eq!(f.fired(), 0);
  }

  #[test]
  fn window_restarts_after_full_release() {
    let mut f = Fixture::spawn(|c| c.within(Duration::from_millis(250)));
    f.set(A, true);
    f.deliver(WindowElapsed);
    f.set(B, true);
    // Re-pressing one switch while the other is still held doesn't restart the window
    f.set(B, false);
    f.set(B, true);
    assert_eq!(f.fired(), 0);

    f.set(A, false);
    f.set(B, false);
    f.set(A, true);
    f.set(B, true);
    assert_eq!(f.fired(), 1);
  }

  #[test]
  fn held_fires_after_duration() {
    let mut f = Fixture::spawn(|c| c.held_for(Duration::from_millis(1250)));
    f.set(A, true);
    f.set(B, true);
    assert_eq!(f.fired(), 0);
    f.deliver(Held);
    assert_eq!(f.fired(), 1);
  }

  #[test]
  fn held_released_early_does_not_fire() {
    let mut f = Fixture::spawn(|c| c.held_for(Duration::from_millis(1250)));
    f.set(A, true);
    f.set(B, true);
    f.set(A, false);
    f.deliver(Held);
    assert_eq!(f.fired(), 0);
  }
}
