use fast_protocol::SwitchState;
use tokio::sync::mpsc;

use crate::metrics::MetricsDb;
use crate::prelude::*;

pub trait AppTracer {
  /// Called once at boot. `metrics` is the database flushed metrics are written to, for reading their history; `None`
  /// when the app has no `metrics_path`, or it couldn't be opened.
  fn init(
    &mut self,
    hardware: &Hardware,
    tx: mpsc::UnboundedSender<TracerControlEvent>,
    metrics: Option<MetricsDb>,
  );
  fn sender(&self) -> mpsc::UnboundedSender<TraceEvent>;
}

#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum TraceEvent {
  Event {
    type_name: &'static str,
    event: Option<serde_json::Value>,
    sender: Option<u64>,
  },
  SystemSpawned {
    id: u64,
    name: &'static str,
    group: GroupId,
  },
  SystemDespawned {
    id: u64,
    group: GroupId,
  },
  SystemGroupSpawned {
    id: GroupId,
    name: &'static str,
  },
  SystemGroupDespawned {
    id: GroupId,
    name: &'static str,
  },
  SystemActiveStateChange {
    id: u64,
    active: bool,
  },
  SystemGroupActiveStateChange {
    id: GroupId,
    active: bool,
  },
  /// A state set in the group changed value. `None` once its owner despawned, so the group no longer sets it.
  StateChange {
    group: GroupId,
    name: &'static str,
    value: Option<serde_json::Value>,
  },
  DriverStateChange {
    driver_id: usize,
    state: DriverState,
  },
  DriverModeChange {
    driver_id: usize,
    mode: DriverMode,
  },
  SwitchStateChange {
    switch_id: usize,
    state: SwitchState,
  },
  LedsRGBChange {
    expansion: u8,
    breakout: Option<u8>,
    states: Vec<(u8, Rgb)>,
  },
  ConfigsInUse {
    configs: Vec<ConfigView>,
  },
  ConfigChange {
    config: ConfigView,
  },
  RoutineStarted {
    routine: &'static str,
    origin: u64,
  },
  /// A routine was started while one of the same type was already running, so the start was ignored
  RoutineRejected {
    routine: &'static str,
    origin: u64,
    running_origin: u64,
  },
  /// The routine was handed to a participant
  RoutineStep {
    routine: &'static str,
    participant: u64,
    /// The participant's group. Copies of a system in several groups share an id.
    group: GroupId,
  },
  RoutineSkipped {
    routine: &'static str,
    participant: &'static str,
    reason: RoutineSkipReason,
  },
  /// The routine's origin is inactive, so it won't move on until the origin is active again
  RoutinePaused {
    routine: &'static str,
  },
  RoutineResumed {
    routine: &'static str,
  },
  /// The current participant halted the routine. It completes once its origin is active.
  RoutineHalted {
    routine: &'static str,
    participant: u64,
    group: GroupId,
  },
  RoutineCompleted {
    routine: &'static str,
    origin: u64,
    completion: RoutineCompletion,
  },
  /// The routine's origin was despawned before it completed
  RoutineAbandoned {
    routine: &'static str,
    origin: u64,
  },
  // TODO: some kind of game-specific state push that is JSON encodable
}

/// Why a routine participant didn't get a step
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum RoutineSkipReason {
  /// No instance in reach of the origin
  Missing,
  Inactive,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum DriverState {
  Fired,
  On,
  Off,
}

/// An operator config and its current value, for tracers to show and edit
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ConfigView {
  pub key: &'static str,
  pub label: &'static str,
  pub desc: &'static str,
  pub type_name: &'static str,
  pub domain: ConfigDomain,
  pub default: ConfigDomainValue,
  pub value: ConfigDomainValue,
  /// The value as the operator menu shows it
  pub display: String,
}

impl ConfigView {
  pub fn new(definition: &ConfigDefinition, value: ConfigDomainValue) -> Self {
    Self {
      key: definition.key,
      label: definition.label,
      desc: definition.desc,
      type_name: definition.type_name,
      domain: definition.domain.clone(),
      default: definition.default,
      value,
      display: definition.format(value),
    }
  }
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

/// Events (commands, really) which are sent from Tracer clients to the main application
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum TracerControlEvent {
  CloseSwitch {
    switch_id: usize,
  },
  OpenSwitch {
    switch_id: usize,
  },
  /// Set an operator config that's in use, as the menu would. Unknown keys and values outside the domain are ignored.
  SetConfig {
    key: String,
    value: ConfigDomainValue,
  },
}
