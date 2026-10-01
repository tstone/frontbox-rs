use frontbox::prelude::Hardware;
use frontbox::prelude::SwitchState;
use frontbox::prelude::app_tracer::{Color, DriverState, TraceEvent};
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
  /// LED colors that changed since the last `Leds`. Sent at a steady rate rather than as they happen: LEDs change
  /// many times a second, in batches, and sending each would swamp slower clients.
  Leds(LedColors),
}

/// Everything the web app needs to render from scratch.
#[derive(Clone, Default, serde::Serialize, TS)]
pub struct Snapshot {
  /// `None` until `AppTracer::init` has been called
  pub hardware: Option<Hardware>,
  pub groups: Vec<SystemGroup>,
  pub switches: BTreeMap<usize, SwitchState>,
  pub drivers: BTreeMap<usize, DriverState>,
  /// The game in progress, if any
  pub game: Option<GameState>,
  /// The last color sent to each LED, by LED name. Colors are in the order sent on the wire, so an LED configured as
  /// GRB has green first.
  pub led_colors: BTreeMap<String, Color>,
  /// Surfaces of the machine to draw, from `WebTracer::plane`
  pub planes: Vec<PlaneView>,
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
  /// The game in progress when this was traced, if any
  pub game: Option<GameState>,
  pub event: TraceEvent,
}

/// Game context, tracked from the turn-based game events
#[derive(Clone, Default, serde::Serialize, TS)]
pub struct GameState {
  /// Zero-based index of the player whose turn it is. `None` until the first turn begins.
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
  /// The most recent events this system emitted, oldest first
  #[ts(as = "Vec<TraceRecord>")]
  pub recent_events: VecDeque<TraceRecord>,
}

/// A surface of the machine to draw, in world space (the cabinet's coordinates, in inches)
#[derive(Clone, serde::Serialize, TS)]
pub struct PlaneView {
  pub name: String,
  /// World position of the plane's local origin
  pub origin: [f32; 3],
  /// World rotation, as a quaternion (x, y, z, w)
  pub rotation: [f32; 4],
  /// Width (local x) and height (local y)
  pub extent: [f32; 2],
  /// Where to fetch the plane's image, if it has one. Its top-left corner sits at the plane's local origin.
  pub image: Option<String>,
  /// How the plane is referred to in the machine's code, e.g. `planes::PLAYFIELD`, for copying positions as code
  pub code: Option<String>,
}

#[derive(Clone, serde::Serialize, TS)]
pub struct LedColors {
  /// By LED name, in the order sent on the wire (see `Snapshot::led_colors`)
  pub colors: BTreeMap<String, Color>,
}
