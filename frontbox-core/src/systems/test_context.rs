use std::any::TypeId;

use tokio::sync::mpsc;

use crate::prelude::app_message::AppMessage;
use crate::prelude::app_tracer::TraceEvent;
use crate::prelude::*;
use crate::systems::state::set_state;

pub struct TestContext {
  pub base: AppState,
  pub groups: Groups,
  tx: mpsc::UnboundedSender<AppMessage>,
  rx: mpsc::UnboundedReceiver<AppMessage>,
}

impl TestContext {
  pub fn insert_system(&mut self, system: impl Into<SystemContainer>) {
    self
      .groups
      .get_mut(&ROOT_GROUP)
      .unwrap()
      .insert(system.into());
  }

  pub fn insert_switch(&mut self, switch: Switch) {
    self.base.switches.insert_switch(switch);
  }

  pub fn insert_driver(&mut self, driver: Driver) {
    self.base.drivers.insert(driver.name, driver);
  }

  pub fn insert_led(&mut self, led: LED) {
    self.base.leds.insert(led.name.to_string(), led);
  }

  pub fn set_led_radius(&mut self, radius: f32) {
    self.base.app_config.led_radius = radius;
  }

  /// Manually set the current state of all switches for test setup
  pub fn update_switch_states(&mut self, states: Vec<SwitchState>) {
    self.base.switches.update_switch_states(states);
  }

  pub fn svc_ctx(&self) -> ServiceContext<'_> {
    ServiceContext::new(&self.base, &self.groups, &self.tx)
  }

  pub fn sys_ctx(&self) -> SystemContext<'_> {
    SystemContext::new(
      &self.base,
      SystemHandle::default(),
      &self.groups,
      &self.tx,
    )
  }

  fn app_messages(&mut self) -> Vec<AppMessage> {
    // Non-blocking so that asserting "nothing was emitted" doesn't hang on an empty channel
    let mut messages = Vec::new();
    while let Ok(msg) = self.rx.try_recv() {
      messages.push(msg);
    }
    messages
  }

  /// A record of all events which were emitted up to this point
  pub fn events_emitted<'a>(&'a mut self) -> Vec<EventBox> {
    let mut events = Vec::new();
    for msg in self.app_messages() {
      match msg {
        AppMessage::EmitEvent(_, e) => events.push(e),
        // the run loop emits this after saving
        AppMessage::ConfigChanged(key) => events.push(EventBox::new(ConfigChanged(key))),
        _ => {}
      }
    }
    events
  }

  /// Routines started up to this point. Downcast to check them.
  pub fn routines_started(&mut self) -> Vec<Box<dyn Event>> {
    self
      .app_messages()
      .into_iter()
      .filter_map(|msg| match msg {
        AppMessage::StartRoutine(start) => Some(start.routine),
        _ => None,
      })
      .collect()
  }

  /// Whether a step of the routine for `E` was completed since the last check
  pub fn routine_step_completed<E: Event>(&mut self) -> bool {
    self
      .app_messages()
      .into_iter()
      .any(|msg| matches!(msg, AppMessage::CompleteRoutineStep(_, type_id) if type_id == TypeId::of::<E>()))
  }

  /// Whether the routine for `E` was halted since the last check
  pub fn routine_halted<E: Event>(&mut self) -> bool {
    self
      .app_messages()
      .into_iter()
      .any(|msg| matches!(msg, AppMessage::HaltRoutine(_, type_id) if type_id == TypeId::of::<E>()))
  }

  /// Applies the states set up to this point, as the run loop would, so they can be read back. Returns the
  /// `StateChanged` events that would be broadcast.
  pub fn apply_states(&mut self) -> Vec<EventBox> {
    let mut changes = Vec::new();
    for msg in self.app_messages() {
      if let AppMessage::SetState(state) = msg {
        changes.extend(set_state(&mut self.groups, state));
      }
    }
    changes
  }

  /// Trace events sent through the run loop up to this point
  pub fn traces(&mut self) -> Vec<TraceEvent> {
    self
      .app_messages()
      .into_iter()
      .filter_map(|msg| match msg {
        AppMessage::TracerEvent(trace) => Some(trace),
        _ => None,
      })
      .collect()
  }
}

impl Default for TestContext {
  fn default() -> Self {
    let (tx, rx) = mpsc::unbounded_channel();
    let mut groups = Groups::new();
    groups.insert(ROOT_GROUP, SystemGroup::new(ROOT_GROUP_NAME));

    // as in `App::run`, so config changes emit `ConfigChanged`
    let mut base = AppState::default();
    base.config.app_sender = Some(tx.clone());

    Self {
      base,
      groups,
      tx,
      rx,
    }
  }
}
