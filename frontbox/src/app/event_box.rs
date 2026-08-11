use std::any::{TypeId, type_name};

use crate::systems::Event;

pub struct EventBox {
  pub event: Box<dyn Event>,
  pub type_id: TypeId,
  pub type_name: &'static str,
}

impl EventBox {
  pub fn new<E: Event>(event: E) -> Self {
    EventBox {
      type_id: event.type_id(),
      type_name: type_name::<E>(),
      event: Box::new(event),
    }
  }

  pub fn try_json(&self) -> Option<serde_json::Value> {
    self.event.to_json()
  }
}
