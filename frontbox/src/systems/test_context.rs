use std::collections::HashMap;

use tokio::sync::mpsc;

use crate::prelude::app_message::AppMessage;
use crate::prelude::*;

pub struct TestContext {
  pub base: BootSnapshot,
  pub groups: Groups,
  tx: mpsc::UnboundedSender<AppMessage>,
  rx: mpsc::UnboundedReceiver<AppMessage>,
}

impl TestContext {
  pub fn insert_system(&mut self, system: impl Into<SystemContainer>) {
    self
      .groups
      .get_mut(ROOT_GROUP)
      .unwrap()
      .insert(system.into());
  }

  pub fn insert_switch(&mut self, switch: Switch) {
    self.base.switches.insert(switch.name, switch);
  }

  pub fn insert_driver(&mut self, driver: Driver) {
    self.base.drivers.insert(driver.name, driver);
  }

  pub fn insert_led(&mut self, led: LED) {
    self.base.leds.insert(led.name.to_string(), led);
  }

  pub fn svc_ctx(&self) -> ServiceContext<'_> {
    ServiceContext::new(&self.base, &self.groups, self.tx.clone())
  }

  pub fn sys_ctx(&self) -> SystemContext<'_> {
    SystemContext::new(
      &self.base,
      SystemHandle::default(),
      &self.groups,
      self.tx.clone(),
    )
  }

  fn app_messages(&mut self) -> Vec<AppMessage> {
    let mut messages = Vec::new();
    self.rx.blocking_recv_many(&mut messages, 100);
    messages
  }

  /// A record of all events which were emitted up to this point
  pub fn events_emitted<'a>(&'a mut self) -> Vec<EventBox> {
    let mut events = Vec::new();
    for msg in self.app_messages() {
      match msg {
        AppMessage::EmitEvent(e) => events.push(e),
        _ => {}
      }
    }
    events
  }
}

impl Default for TestContext {
  fn default() -> Self {
    let (tx, rx) = mpsc::unbounded_channel();
    let mut groups = HashMap::new();
    groups.insert(ROOT_GROUP, SystemGroup::new());

    Self {
      base: BootSnapshot::default(),
      groups,
      tx,
      rx,
    }
  }
}
