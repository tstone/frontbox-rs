use frontbox::prelude::ConfigDomainValue;
use frontbox::prelude::DriverMode;
use frontbox::prelude::Hardware;
use frontbox::prelude::SwitchState;
use frontbox::prelude::app_tracer::{Color, ConfigView, DriverState, TraceEvent};
use frontbox_metrics::CounterTotal;
use std::collections::{BTreeMap, VecDeque};
use ts_rs::TS;

/// Messages pushed from the backend to the web app over websockets
#[derive(Clone, serde::Serialize, TS)]
#[serde(tag = "type")]
pub enum ServerMessage {
  Init(Snapshot),
  Trace(TraceRecord),
  Leds(LedColors),
  Configs(ConfigList),
  /// Sent only to the client that asked, in reply to `QueryMetrics`
  MetricTotals(MetricTotals),
}

#[derive(Clone, Default, serde::Serialize, TS)]
pub struct Snapshot {
  /// `None` until `AppTracer::init` has been called
  pub hardware: Option<Hardware>,
  pub groups: Vec<SystemGroup>,
  pub switches: BTreeMap<usize, SwitchState>,
  pub drivers: BTreeMap<usize, DriverState>,
  /// Modes drivers were re-configured with while running, by id. A driver not here is in its defined mode.
  pub driver_modes: BTreeMap<usize, DriverMode>,
  pub game: Option<GameState>,
  /// The last color sent to each LED, by name, in wire channel order (an LED wired GRB has green first)
  pub led_colors: BTreeMap<String, Color>,
  pub planes: Vec<PlaneView>,
  /// Operator configs in use, by key, with their current values
  pub configs: BTreeMap<&'static str, ConfigView>,
  #[ts(as = "Vec<TraceRecord>")]
  pub log: VecDeque<TraceRecord>, // oldest first
}

#[derive(Clone, serde::Serialize, TS)]
pub struct TraceRecord {
  /// Message number, unique per console server
  pub seq: u64,
  pub at_ms: u64,
  pub game: Option<GameState>,
  pub event: TraceEvent,
}

#[derive(Clone, Default, serde::Serialize, TS)]
pub struct GameState {
  pub player: Option<u8>,
  pub turn: Option<u8>,
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
  #[ts(as = "Vec<TraceRecord>")]
  pub recent_events: VecDeque<TraceRecord>,
}

#[derive(Clone, serde::Serialize, TS)]
pub struct PlaneView {
  pub name: String,
  pub origin: [f32; 3],
  pub rotation: [f32; 4],
  pub extent: [f32; 2],
  pub image: Option<String>,
  /// e.g. `planes::PLAYFIELD`, when added with `console_plane!`
  pub code: Option<String>,
}

#[derive(Clone, serde::Serialize, TS)]
pub struct LedColors {
  pub colors: BTreeMap<String, Color>,
}

/// Operator configs that were first used or changed
#[derive(Clone, serde::Serialize, TS)]
pub struct ConfigList {
  pub configs: Vec<ConfigView>,
}

/// Metric history read from the metrics database. Trails what's been recorded by up to one flush.
#[derive(Clone, serde::Serialize, TS)]
pub struct MetricTotals {
  /// Whether the app keeps a metrics database. Without one there's no history.
  pub stored: bool,
  pub counters: Vec<CounterTotal>,
}

/// Messages from the web app to the console server
#[derive(Clone, serde::Deserialize, TS)]
#[serde(tag = "type")]
pub enum ClientMessage {
  /// Hold a switch closed, or let it open, as if it were pressed on the machine
  SetSwitch { switch_id: usize, closed: bool },
  /// Set an operator config, as the operator menu would
  SetConfig { key: String, value: ConfigDomainValue },
  /// Ask for every counter's total. Answered with `MetricTotals`.
  QueryMetrics,
}
