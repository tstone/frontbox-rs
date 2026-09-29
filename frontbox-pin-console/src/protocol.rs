use frontbox::prelude::Hardware;
use frontbox::prelude::SwitchState;
use frontbox::prelude::app_tracer::{DriverState, TraceEvent};
use std::collections::{BTreeMap, VecDeque};
use ts_rs::TS;

/// Messages pushed from the console server to the web app over websockets
#[derive(Clone, serde::Serialize, TS)]
#[serde(tag = "type")]
pub enum ServerMessage {
  /// Full, known state, sent when a new client connects
  Init(Snapshot),
  /// Individual events, as they happen
  Trace(TraceRecord),
}

/// Everything the web app needs to render from scratch.
#[derive(Clone, Default, serde::Serialize, TS)]
pub struct Snapshot {
  /// `None` until `AppTracer::init` has been called
  pub hardware: Option<Hardware>,
  pub groups: Vec<SystemGroup>,
  pub switches: BTreeMap<usize, SwitchState>,
  pub drivers: BTreeMap<usize, DriverState>,
  /// Most recent trace records, oldest first
  #[ts(as = "Vec<TraceRecord>")]
  pub log: VecDeque<TraceRecord>,
}

#[derive(Clone, serde::Serialize, TS)]
pub struct TraceRecord {
  /// Monotonic sequence number, unique per console server
  pub seq: u64,
  /// Milliseconds since the console server started
  pub at_ms: u64,
  pub event: TraceEvent,
}

#[derive(Clone, serde::Serialize, TS)]
pub struct SystemGroup {
  pub key: &'static str,
  pub active: bool,
  pub systems: Vec<System>,
}

#[derive(Clone, serde::Serialize, TS)]
pub struct System {
  pub id: u64,
  pub name: &'static str,
  pub active: bool,
}
