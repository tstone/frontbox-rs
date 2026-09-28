use fast_protocol::SwitchState;
use tokio::sync::mpsc;

use crate::prelude::*;

pub trait AppTracer {
  fn init(&mut self, hardware: &Hardware);
  fn sender(&self) -> mpsc::UnboundedSender<TraceEvent>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum TraceEvent {
  Event {
    type_name: &'static str,
    interrupts: Vec<InterruptEvaluation>,
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    event: Option<serde_json::Value>,
  },
  SystemSpawned {
    id: u64,
    name: &'static str,
    parent_key: &'static str,
  },
  SystemDespawned {
    id: u64,
    parent_key: &'static str,
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
  },
  DriverStateChange {
    driver_id: usize,
    state: DriverState,
  },
  SwitchStateChange {
    switch_id: usize,
    state: SwitchState,
  },
  // TODO: some kind of game-specific state push that is JSON encodable
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InterruptEvaluation {
  pub interrupter: u64,
  pub result: InterruptResult,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum DriverState {
  Fired,
  On,
  Off,
}

pub(crate) struct TracerSenders {
  txs: Vec<mpsc::UnboundedSender<TraceEvent>>,
}

impl TracerSenders {
  pub fn new(txs: Vec<mpsc::UnboundedSender<TraceEvent>>) -> Self {
    Self { txs }
  }

  pub fn send(&self, event: TraceEvent) {
    for tracer in &self.txs {
      tracer.send(event.clone()).ok();
    }
  }
}
