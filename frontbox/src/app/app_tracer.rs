use std::any::TypeId;
use tokio::sync::mpsc;

use crate::prelude::*;

pub trait AppTracer {
  fn start(&mut self);
  fn sender(&self) -> mpsc::UnboundedSender<TraceEvent>;
}

pub enum TraceEvent {
  Event {
    type_id: TypeId,
    type_name: &'static str,
    interrupts: Vec<InterruptEvaluation>,
    event: Option<serde_json::Value>
  },
  SystemSpawned {
    id: u64,
    name: &'static str,
    parent_key: &'static str
  },
  SystemDespawned {
    id: u64,
    parent_key: &'static str
  },
  SystemGroupSpawned {
    key: &'static str,
  },
  SystemGroupDespawned {
    key: &'static str,
  },
  SystemActiveStateChange {
    id: u64,
    active: bool,
  },
  SystemGroupActiveStateChange {
    key: &'static str,
    active: bool,
  }
}

#[derive(Debug, Clone)]
pub struct InterruptEvaluation {
  pub interrupter: u64,
  pub result: InterruptResult,
}