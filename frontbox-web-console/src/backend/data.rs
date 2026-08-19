use frontbox::prelude::app_tracer::InterruptEvaluation;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SystemGroup {
  pub key: &'static str,
  pub systems: Vec<System>,
  pub active: bool,
}

impl SystemGroup {
  pub fn new(key: &'static str) -> Self {
    Self {
      key,
      systems: Vec::new(),
      active: true,
    }
  }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct System {
  pub name: &'static str,
  pub id: u64,
  pub active: bool,
}

impl System {
  pub fn new(name: &'static str, id: u64) -> Self {
    Self {
      name,
      id,
      active: true,
    }
  }
}

#[derive(Debug, Clone)]
pub struct SystemEvent {
  pub type_name: &'static str,
  #[allow(unused)]
  pub interrupts: Vec<InterruptEvaluation>,
  pub event: Option<serde_json::Value>,
}

impl SystemEvent {
  pub fn new(
    type_name: &'static str,
    interrupts: Vec<InterruptEvaluation>,
    event: Option<serde_json::Value>,
  ) -> Self {
    Self {
      type_name,
      interrupts,
      event,
    }
  }
}
