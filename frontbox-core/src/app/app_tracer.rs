use fast_protocol::SwitchState;
use tokio::sync::mpsc;

use crate::metrics::MetricsDb;
use crate::prelude::*;

pub trait AppTracer {
  /// Called once at boot. `config` is what the app was booted with. `metrics` is the database flushed metrics are written to, for reading their history; `None`
  /// when the app has no `metrics_path`, or it couldn't be opened.
  fn init(
    &mut self,
    hardware: &Hardware,
    config: &AppConfig,
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
    /// The system that spawned it, or `None` at the top level
    parent: Option<u64>,
    /// Whether it started active, which it doesn't under an inactive parent
    active: bool,
    /// Whether it's a `Group`, in which case `name` is the group's own name
    group: bool,
    /// What it's for, from [`System::description`](crate::systems::System::description)
    description: Option<&'static str>,
  },
  SystemDespawned {
    id: u64,
    cause: DespawnCause,
  },
  SystemActiveStateChange {
    id: u64,
    active: bool,
    cause: ActivityCause,
  },
  /// A state changed value. `None` once its owner despawned, so no system sets it.
  StateChange {
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
  /// The participant's step is over
  RoutineStepEnded {
    routine: &'static str,
    participant: u64,
    ending: StepEnding,
  },
  /// The current participant halted the routine. It completes once its origin is active.
  RoutineHalted {
    routine: &'static str,
    participant: u64,
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
  /// A system reported its status, for tracers only, with
  /// [`SystemContext::report_status`](crate::systems::SystemContext::report_status)
  SystemStatusReported {
    id: u64,
    status: serde_json::Value,
  },
}

/// Why a routine participant didn't get a step
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum RoutineSkipReason {
  /// No system of its type is running
  Missing,
  Inactive,
}

/// Why a system became active, or stopped being active
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum ActivityCause {
  /// Its parent activated or deactivated it
  Parent { id: u64 },
  /// An ancestor became active or stopped, and the change reached it. `id` is the ancestor where it started.
  Ancestor { id: u64 },
  /// Its own `is_active` changed. `during` is what was being delivered when the framework noticed: an event's type name,
  /// or `"tick"`. Usually a state change just before it is what changed it.
  IsActive { during: Option<&'static str> },
}

/// Why a system was despawned
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum DespawnCause {
  /// It despawned itself
  Itself,
  /// One of its handlers panicked
  Panicked,
  /// It replaced itself with another system
  Replaced,
  /// Its parent despawned it
  Parent { id: u64 },
  /// An ancestor was despawned. `id` is the ancestor where it started.
  Ancestor { id: u64 },
}

/// How a routine participant's step came to an end
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum StepEnding {
  /// It completed its step
  Completed,
  /// It halted the routine
  Halted,
  /// It was despawned or deactivated mid-step
  Stopped,
  /// The routine's origin cancelled it
  Cancelled,
  /// The routine's origin was despawned
  Abandoned,
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

  /// Send the event `build` returns, calling it only when a tracer is attached. For events that are costly to build.
  pub fn send_lazy(&self, build: impl FnOnce() -> TraceEvent) {
    if !self.txs.is_empty() {
      self.send(build());
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
  /// Play a sound from the content folder, as a scene previewed in the console reaches one of its sounds. Sent on to
  /// systems as [`SoundPreviewRequested`].
  PlaySound {
    sound: String,
    volume: f32,
    skip: f32,
    length: Option<f32>,
  },
}

/// A tracer asked for a sound from the content folder to be played, at `volume` (1.0 is as recorded), from `skip` seconds
/// in, and cut off after `length` seconds if given. `SoundSystem` plays it.
#[derive(Debug, Clone, serde::Serialize, Event)]
pub struct SoundPreviewRequested {
  pub sound: String,
  pub volume: f32,
  pub skip: f32,
  pub length: Option<f32>,
}
