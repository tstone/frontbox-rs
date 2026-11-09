use frontbox_core::prelude::ConfigDomainValue;
use frontbox_core::prelude::DriverMode;
use frontbox_core::prelude::GroupId;
use frontbox_core::prelude::Hardware;
use frontbox_core::prelude::Rgb;
use frontbox_core::prelude::SwitchState;
use frontbox_core::prelude::app_tracer::{ConfigView, DriverState, TraceEvent};
use frontbox_dmd::DmdScene;
use frontbox_gfx::scene::file::{self, FormatError};
use frontbox_gfx::scene::GfxComposition;
use frontbox_led::LedScene;
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
  /// Sent only to the client that asked, in reply to `RenderScene`
  SceneRendered(RenderedScene),
  /// Sent only to the client that asked, when the scene in a `RenderScene` can't be rendered
  SceneRenderFailed { error: String },
  /// Sent only to the client that asked, when the scene in a `RenderClip` can't be rendered
  SceneClipFailed { id: u32, error: String },
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
  /// Routines in progress, oldest first
  pub routines: Vec<RoutineProgress>,
  /// The last color sent to each LED, by name, in wire channel order (an LED wired GRB has green first)
  pub led_colors: BTreeMap<String, Rgb>,
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
pub struct RoutineProgress {
  pub routine: &'static str,
  pub origin: u64,
  /// The system whose step is in progress
  pub current: Option<RoutineParticipant>,
}

#[derive(Clone, serde::Serialize, TS)]
pub struct RoutineParticipant {
  pub id: u64,
  /// Copies of a system in several groups share an id
  pub group: GroupId,
}

#[derive(Clone, serde::Serialize, TS)]
pub struct SystemGroup {
  pub id: GroupId,
  /// For display; names don't need to be unique
  pub name: &'static str,
  pub active: bool,
  pub systems: Vec<System>,
  /// The states set in this group, by type name, as JSON. Reading one falls back to the root's when it isn't here.
  pub states: BTreeMap<&'static str, serde_json::Value>,
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
  /// e.g. `planes::PLAYFIELD`, when added with `app_plane!`
  pub code: Option<String>,
}

#[derive(Clone, serde::Serialize, TS)]
pub struct LedColors {
  pub colors: BTreeMap<String, Rgb>,
}

/// A scene's colors at each point, row by row from the top left
#[derive(Clone, serde::Serialize, TS)]
pub struct RenderedScene {
  /// The scene that was rendered
  pub scene: SceneDefinition,
  /// The same scene as RON
  pub ron: String,
  pub width: usize,
  pub height: usize,
  /// Red, green, blue, and alpha for each point, as a canvas's `ImageData` takes them
  pub colors: Vec<u8>,
  /// The part of the scene those points show, in scene units
  pub area: RenderedArea,
  /// Where each layer is drawn, as a closed loop of points in scene units
  pub outlines: Vec<Vec<[f32; 2]>>,
  /// Each layer's shape as it stands, for the editor's handles. `None` for a layer without one.
  pub shapes: Vec<Option<RenderedShape>>,
  /// Where each layer's mask is drawn. Empty for a layer without one.
  pub mask_outlines: Vec<Vec<[f32; 2]>>,
  /// Each layer's mask's shape as it stands. `None` for a layer without one.
  pub mask_shapes: Vec<Option<RenderedShape>>,
  /// What each LED the scene lands on would show, by name. Unlike colors from the machine these are in red, green, blue
  /// order, however the LED is wired.
  pub led_colors: BTreeMap<String, Rgb>,
}

/// A shape as it stands at a moment
#[derive(Clone, serde::Serialize, TS)]
pub struct RenderedShape {
  /// What each transform does, in the order they apply: where it takes x's axis, y's axis, and the origin
  pub transforms: Vec<[f32; 6]>,
  /// The corners of its bounds in its own space
  pub min: [f32; 2],
  pub max: [f32; 2],
  /// The points of a polygon, route or bezier in its own space
  pub points: Vec<[f32; 2]>,
  /// A bezier's handles, each from its point
  pub handles: Vec<[f32; 2]>,
  /// How far a ring's hole reaches from its center, in its own space
  pub hole: f32,
  /// Its edge in the scene, as a closed loop of points, or its line for a polyline or bezier, which doesn't close
  pub outline: Vec<[f32; 2]>,
  /// For each transform, the path it moves the shape along, in the values the transform takes. `None` for one that
  /// doesn't.
  pub paths: Vec<Option<RenderedShape>>,
}

/// What the console says about an image in the content folder, so an editor can offer its frames
#[derive(Clone, serde::Serialize, TS)]
pub struct ImageInfo {
  pub width: u32,
  pub height: u32,
  /// 1 for an image that isn't animated
  pub frames: usize,
  /// Seconds it takes to play, by what the file says
  pub duration: f32,
}

/// How to read a clip, which is the one thing sent as a binary message: this as JSON, its length before it as a
/// little-endian `u32`. After it come the frames' points, red, green, blue, and alpha as for `RenderedScene::colors`,
/// then the frames' LEDs, as red, green, and blue in the order of `leds`.
#[derive(Clone, serde::Serialize, TS)]
pub struct RenderedClip {
  /// The `RenderClip` this answers
  pub id: u32,
  /// Evenly spaced from the start of a cycle to its end, both included
  pub frames: usize,
  pub width: usize,
  pub height: usize,
  /// The name of each LED the scene lands on
  pub leds: Vec<String>,
}

/// A rectangle in a scene
#[derive(Clone, serde::Serialize, TS)]
pub struct RenderedArea {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
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
  SetConfig {
    key: String,
    value: ConfigDomainValue,
  },
  /// Ask for every counter's total. Answered with `MetricTotals`.
  QueryMetrics,
  /// Render a moment of a scene. Answered with `SceneRendered` or `SceneRenderFailed`.
  RenderScene {
    source: SceneSource,
    size: SceneSize,
    /// Seconds into a cycle of the scene
    time: f32,
    /// Which cycle, counting from zero. With none, it's one from the middle of the scene, to show it as it looks partway
    /// through play.
    cycle: Option<u32>,
  },
  /// Render a whole cycle of a scene for the client to play. Answered with a clip, or `SceneClipFailed`.
  RenderClip {
    id: u32,
    source: SceneSource,
    size: SceneSize,
    /// Which cycle, counting from zero
    cycle: u32,
  },
}

/// A scene to render, as either of the forms the console edits it in
#[derive(Clone, serde::Deserialize, TS)]
pub enum SceneSource {
  /// As written in a file
  Ron(String),
  /// As edited field by field
  Definition(SceneDefinition),
  /// Drawn as `shown`, for trying a change out without making it. `scene` is what's given back in both forms.
  Preview { scene: SceneDefinition, shown: SceneDefinition },
}

/// A scene file, whichever kind it says it is
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, TS)]
pub enum SceneDefinition {
  Led(LedScene),
  Dmd(DmdScene),
}

impl SceneDefinition {
  pub fn load(source: &str) -> Result<Self, FormatError> {
    let scene: Self = file::load(source)?;
    file::check_version(scene.version())?;
    scene.check().map_err(FormatError::Invalid)?;
    Ok(scene)
  }

  pub fn save(&self) -> Result<String, FormatError> {
    file::save(self)
  }

  /// Checks what a definition's types can't say for themselves
  pub fn check(&self) -> Result<(), String> {
    match self {
      Self::Led(scene) => scene.check().map_err(|err| err.to_string()),
      Self::Dmd(scene) => scene.check().map_err(|err| err.to_string()),
    }
  }

  fn version(&self) -> u32 {
    match self {
      Self::Led(scene) => scene.version,
      Self::Dmd(scene) => scene.version,
    }
  }

  /// What's drawn, leaving out where
  pub fn composition(&self) -> GfxComposition {
    match self {
      Self::Led(scene) => scene.composition(),
      Self::Dmd(scene) => scene.composition.clone(),
    }
  }
}

/// How many points to sample a scene at. The kind of scene decides which of these applies.
#[derive(Clone, Copy, serde::Deserialize, TS)]
pub struct SceneSize {
  /// LEDs along the strip, for an LED scene's `Sequence` layers
  pub leds: usize,
  /// Points across, for an `Area` viewport
  pub width: usize,
  /// Points down, for an `Area` viewport
  pub height: usize,
}

