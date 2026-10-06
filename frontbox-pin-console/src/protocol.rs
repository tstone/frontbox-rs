use frontbox::prelude::Hardware;
use frontbox::prelude::SwitchState;
use frontbox::prelude::app_tracer::{Color, DriverState, TraceEvent};
use std::collections::{BTreeMap, VecDeque};
use ts_rs::TS;

/// Messages pushed from the backend to the web app over websockets
#[derive(Clone, serde::Serialize, TS)]
#[serde(tag = "type")]
pub enum ServerMessage {
  Init(Snapshot),
  Trace(TraceRecord),
  Leds(LedColors),
}

#[derive(Clone, Default, serde::Serialize, TS)]
pub struct Snapshot {
  /// `None` until `AppTracer::init` has been called
  pub hardware: Option<Hardware>,
  pub groups: Vec<SystemGroup>,
  pub switches: BTreeMap<usize, SwitchState>,
  pub drivers: BTreeMap<usize, DriverState>,
  pub game: Option<GameState>,
  /// The last color sent to each LED, by name, in wire channel order (an LED wired GRB has green first)
  pub led_colors: BTreeMap<String, Color>,
  pub planes: Vec<PlaneView>,
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
