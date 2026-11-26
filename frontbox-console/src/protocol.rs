use frontbox_core::prelude::ConfigDomainValue;
use frontbox_core::prelude::DriverMode;
use frontbox_core::prelude::Hardware;
use frontbox_core::prelude::Rgb;
use frontbox_core::prelude::SwitchState;
use frontbox_core::prelude::app_tracer::{
  ActivityCause, ConfigView, DespawnCause, DriverState, StepEnding, TraceEvent,
};
use frontbox_metrics::CounterTotal;
use frontbox_scene::Scene;
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
  SceneRenderFailed {
    error: String,
  },
  /// Sent only to the client that asked, when the scene in a `RenderClip` can't be rendered
  SceneClipFailed {
    id: u32,
    error: String,
  },
}

#[derive(Clone, Default, serde::Serialize, TS)]
pub struct Snapshot {
  /// `None` until `AppTracer::init` has been called
  pub hardware: Option<Hardware>,
  /// Every running system, in spawn order. Each names its parent, which together make the system tree.
  pub systems: Vec<System>,
  /// Every state that's set, by type name, as JSON. States have one value for the whole machine.
  pub states: BTreeMap<&'static str, serde_json::Value>,
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
  /// Displays mirrored in the console. Their frames are binary messages naming them by their place here.
  pub displays: Vec<DisplayView>,
  /// Operator configs in use, by key, with their current values
  pub configs: BTreeMap<&'static str, ConfigView>,
  /// Whether a game starting clears the history, as `ClientMessage::ClearHistory` does
  pub clear_on_game_start: bool,
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
  /// The id of the system whose step is in progress
  pub current: Option<u64>,
}

#[derive(Clone, serde::Serialize, TS)]
pub struct System {
  pub id: u64,
  /// Its type's name, or a `Group`'s own name
  pub name: &'static str,
  /// The system that spawned it, or `None` at the top level
  pub parent: Option<u64>,
  pub active: bool,
  /// Whether it's a `Group`, named by its own name rather than its type's
  pub group: bool,
  /// What it's for, when it says
  pub description: Option<&'static str>,
  /// When it was spawned, in milliseconds since the console started (as `TraceRecord::at_ms`)
  pub spawned_ms: u64,
  /// When it was despawned. A despawned system is kept for a while, to look back at.
  pub despawned_ms: Option<u64>,
  /// What happened to it, oldest first. Only the latest are kept.
  #[ts(as = "Vec<HistoryEntry>")]
  pub history: VecDeque<HistoryEntry>,
  #[ts(as = "Vec<TraceRecord>")]
  pub recent_events: VecDeque<TraceRecord>,
  /// The latest statuses it reported (`SystemStatusReported`), oldest first. The last is its status now.
  #[ts(as = "Vec<TraceRecord>")]
  pub recent_statuses: VecDeque<TraceRecord>,
}

/// Something that happened to a system, for its history. Times are in milliseconds since the console started.
#[derive(Clone, Debug, PartialEq, serde::Serialize, TS)]
#[serde(tag = "kind")]
pub enum HistoryEntry {
  Spawned {
    at_ms: u64,
    active: bool,
  },
  ActivityChanged {
    at_ms: u64,
    seq: u64,
    active: bool,
    cause: ActivityCause,
  },
  /// It held a routine's step, from `started_ms` until `ended_ms`, or still does
  RoutineStep {
    routine: &'static str,
    started_ms: u64,
    ended_ms: Option<u64>,
    ending: Option<StepEnding>,
  },
  /// It emitted an event, by type name. `seq` finds it in the log.
  Emitted {
    at_ms: u64,
    seq: u64,
    event: &'static str,
  },
  /// It reported its status. `seq` finds it in `System::recent_statuses` or the log.
  StatusReported {
    at_ms: u64,
    seq: u64,
  },
  /// Its history from before was cleared (`ConsoleHub::clear_history`). It was `active` then, and a routine step it
  /// still held is kept.
  HistoryCleared {
    at_ms: u64,
    active: bool,
  },
  Despawned {
    at_ms: u64,
    cause: DespawnCause,
  },
}

#[derive(Clone, serde::Serialize, TS)]
pub struct PlaneView {
  pub name: String,
  pub origin: [f32; 3],
  pub rotation: [f32; 4],
  pub extent: [f32; 2],
  /// The corners of its edge, in its own coordinates: the rectangle over `extent` unless it has an outline
  pub corners: Vec<[f32; 2]>,
  pub image: Option<String>,
  /// e.g. `planes::PLAYFIELD`, when added with `app_plane!`
  pub code: Option<String>,
}

/// The first byte of a binary message, which says what the rest is
pub const BINARY_CLIP: u8 = 0;
/// A display's frame: then a byte for its place in `Snapshot::displays`, then red, green, and blue for each pixel,
/// row-major from the top left
pub const BINARY_DISPLAY_FRAME: u8 = 1;

/// A display mirrored in the console
#[derive(Clone, serde::Serialize, TS)]
pub struct DisplayView {
  pub name: String,
  pub width: u32,
  pub height: u32,
  /// Where it is on the machine, to draw it there. `None` draws it on its own.
  pub placement: Option<DisplayPlacement>,
}

/// A rectangle on a plane, in the plane's units, from `origin` to `origin + size`
#[derive(Clone, serde::Serialize, TS)]
pub struct DisplayPlacement {
  pub plane: String,
  pub origin: [f32; 2],
  pub size: [f32; 2],
}

#[derive(Clone, serde::Serialize, TS)]
pub struct LedColors {
  pub colors: BTreeMap<String, Rgb>,
}

/// A moment of a scene, on everything it draws on
#[derive(Clone, serde::Serialize, TS)]
pub struct RenderedScene {
  /// The scene that was rendered
  pub scene: Scene,
  /// The same scene as RON
  pub ron: String,
  /// The strip of its `Sequence` light layers, if it has any, then each display it or the reference scene draws on
  pub canvases: Vec<RenderedCanvas>,
  /// Where each layer is drawn, as closed loops of points in scene units: one for each copy of its shape
  pub outlines: Vec<Vec<Vec<[f32; 2]>>>,
  /// Each layer's shape as it stands, for the editor's handles. `None` for a layer without one.
  pub shapes: Vec<Option<RenderedShape>>,
  /// For each layer, the area of each part of it that goes behind another, as it stands, in the layer's order. `None`
  /// for a collapsed one.
  pub behind: Vec<Vec<Option<RenderedShape>>>,
  /// What each light the scene lands on would show, by name. Unlike colors from the machine these are in red, green, blue
  /// order, however the LED is wired.
  pub light_colors: BTreeMap<String, Rgb>,
  /// The graph each layer is limited to, built from the lights it lands on. `None` for a layer without one.
  pub graphs: Vec<Option<RenderedGraph>>,
  /// Why the reference scene asked for couldn't be drawn, which leaves it out of the render
  pub reference_error: Option<String>,
}

/// The colors a render drew at each point of one thing a scene draws on, row by row from the top left
#[derive(Clone, serde::Serialize, TS)]
pub struct RenderedCanvas {
  /// The display, by name, or `None` for the strip of `Sequence` light layers
  pub display: Option<String>,
  pub width: usize,
  pub height: usize,
  /// Red, green, blue, and alpha for each point, as a canvas's `ImageData` takes them
  pub colors: Vec<u8>,
  /// The part of the scene those points show, in scene units
  pub area: RenderedArea,
}

/// A layer's graph: its nodes, where each samples the scene in scene units, and its edges
#[derive(Clone, serde::Serialize, TS)]
pub struct RenderedGraph {
  /// The light each node is, by name
  pub names: Vec<String>,
  pub nodes: Vec<[f32; 2]>,
  /// Each edge once, by the indexes of its two nodes
  pub edges: Vec<[usize; 2]>,
  /// Seconds each of the layer's traversals takes from leaving `from` to its last visit, in the layer's order, however
  /// long the cycle is. `None` for one that visits nothing.
  pub lengths: Vec<Option<f32>>,
}

/// A shape as it stands at a moment
#[derive(Clone, serde::Serialize, TS)]
pub struct RenderedShape {
  /// What each transform does, in the order they apply: where it takes x's axis, y's axis, and the origin
  pub transforms: Vec<[f32; 6]>,
  /// The corners of its bounds in its own space
  pub min: [f32; 2],
  pub max: [f32; 2],
  /// The points of a polygon, route or bezier in its own space, or where each handle of an external shape's generator
  /// is
  pub points: Vec<[f32; 2]>,
  /// A bezier's handles, each from its point
  pub handles: Vec<[f32; 2]>,
  /// Where its center is going into each transform, in the space the ones before it put the shape in, then in the
  /// scene after the last. A rotate turns around it.
  pub pivots: Vec<[f32; 2]>,
  /// For each transform, the center of a mirror, revolve, or bisect, in the space the ones before it put the shape in.
  /// `None` for other transforms.
  pub centers: Vec<Option<[f32; 2]>>,
  /// For each scale, the point it keeps still, in the space the transforms before it put the shape in. `None` for other
  /// transforms.
  pub anchors: Vec<Option<[f32; 2]>>,
  /// For each mirror or bisect, the direction its line runs through its center, unit length. `None` for other
  /// transforms.
  pub lines: Vec<Option<[f32; 2]>>,
  /// For each shift, the ends of the shape along its line and where it moves the middle to, in the space the transforms
  /// before it put the shape in. `None` for other transforms.
  pub shifts: Vec<Option<RenderedShift>>,
  /// Its own edge in the scene, without the copies and cuts of a mirror, revolve, or bisect, as a closed loop of
  /// points, or its line for a polyline or bezier, which doesn't close
  pub outline: Vec<[f32; 2]>,
  /// For each transform, the path it moves the shape along, in the values the transform takes. `None` for one that
  /// doesn't.
  pub paths: Vec<Option<RenderedShape>>,
}

/// Where a shift moves a shape
#[derive(Clone, serde::Serialize, TS)]
pub struct RenderedShift {
  pub back: [f32; 2],
  pub forward: [f32; 2],
  pub middle: [f32; 2],
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

/// A font a scene can use, so an editor can offer sizes it draws crisply at
#[derive(Clone, serde::Serialize, TS)]
pub struct FontInfo {
  pub name: String,
  /// For a font drawn on a pixel grid, the size that's one scene unit to its pixels. It's crisp at whole multiples of it.
  pub pixel_size: Option<f32>,
  /// For a pixel font, how many of its pixels tall its capital letters are
  pub cap_height: Option<f32>,
}

/// A shape generator the app has added, which a scene's `External` shape can name, so an editor can offer it and a
/// field for each of its settings
#[derive(Clone, serde::Serialize, TS)]
pub struct ShapeGeneratorInfo {
  /// What a scene names it by
  pub name: String,
  /// Whether it has a path a sweep can follow
  pub has_path: bool,
  /// Whether its outline changes with time when its settings don't
  pub is_animated: bool,
  pub settings: Vec<GeneratorSettingInfo>,
  /// Points an editor shows a handle on, in the order a `RenderedShape` of it has them as its `points`
  pub handles: Vec<GeneratorHandleInfo>,
}

/// An image generator the app has added, which a scene's `External` fill can name, so an editor can offer it and a
/// field for each of its settings
#[derive(Clone, serde::Serialize, TS)]
pub struct ImageGeneratorInfo {
  /// What a scene names it by
  pub name: String,
  /// Whether its image changes with time when its settings don't
  pub is_animated: bool,
  pub settings: Vec<GeneratorSettingInfo>,
}

/// A point two of a shape generator's settings make, which an editor shows a handle on
#[derive(Clone, serde::Serialize, TS)]
pub struct GeneratorHandleInfo {
  pub name: String,
  /// The names of the settings that are its x and its y
  pub x: String,
  pub y: String,
}

/// One setting of a shape or image generator, which a scene sets as a number
#[derive(Clone, serde::Serialize, TS)]
pub struct GeneratorSettingInfo {
  pub name: String,
  /// What it is when a scene leaves it out
  pub default: f32,
  /// The lowest and highest it can be, if it has them
  pub range: Option<(f32, f32)>,
}

/// How to read a clip, which is the one thing sent as a binary message: this as JSON, its length before it as a
/// little-endian `u32`. After it come each canvas's frames, canvas by canvas, with red, green, blue, and alpha for each
/// point as in `RenderedCanvas::colors`, then the frames' lights, as red, green, and blue in the order of `lights`.
#[derive(Clone, serde::Serialize, TS)]
pub struct RenderedClip {
  /// The `RenderClip` this answers
  pub id: u32,
  /// Evenly spaced from the start of a cycle to its end, both included
  pub frames: usize,
  /// What each canvas is of, in the order their frames come, as in `RenderedScene::canvases`
  pub canvases: Vec<ClipCanvas>,
  /// The name of each light the scene lands on
  pub lights: Vec<String>,
  /// The sounds its sound layers start in the cycle, in the order they start
  pub sounds: Vec<ClipSound>,
}

/// One of the canvases in a [`RenderedClip`]
#[derive(Clone, serde::Serialize, TS)]
pub struct ClipCanvas {
  /// The display, by name, or `None` for the strip of `Sequence` light layers
  pub display: Option<String>,
  pub width: usize,
  pub height: usize,
}

/// A sound starting in a [`RenderedClip`]
#[derive(Clone, serde::Serialize, TS)]
pub struct ClipSound {
  /// Seconds into the cycle
  pub at: f32,
  /// The sound, by its name in the content folder
  pub sound: String,
  /// 1.0 is as recorded
  pub volume: f32,
  /// Seconds cut from the beginning of the sound
  pub skip: f32,
  /// Seconds it plays before it's cut off, or `None` to play to its end
  pub length: Option<f32>,
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
  /// Clear the log, systems' histories, and despawned systems, keeping what's current. Every client is sent a new
  /// `Init`.
  ClearHistory,
  /// Have a game starting clear the history, or not
  SetClearOnGameStart { clear: bool },
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
    #[serde(default)]
    reference: Option<ReferenceScene>,
    /// What the scene's random choices come from, as each playback on the machine has its own. The same seed gives the
    /// same choices, so the moments of one play of the scene agree.
    #[serde(default)]
    seed: u32,
  },
  /// Render a whole cycle of a scene for the client to play. Answered with a clip, or `SceneClipFailed`.
  RenderClip {
    id: u32,
    source: SceneSource,
    size: SceneSize,
    /// Which cycle, counting from zero
    cycle: u32,
    #[serde(default)]
    reference: Option<ReferenceScene>,
    /// As for `RenderScene`
    #[serde(default)]
    seed: u32,
  },
  /// Play a sound from the content folder on the machine, as a scene preview reaches the start of one of its sounds. Nothing plays if
  /// the app has no sound system.
  PlaySound {
    sound: String,
    volume: f32,
    skip: f32,
    length: Option<f32>,
  },
}

/// A scene to render, as either of the forms the console edits it in
#[derive(Clone, serde::Deserialize, TS)]
pub enum SceneSource {
  /// As written in a file
  Ron(String),
  /// As edited field by field
  Definition(Scene),
  /// Drawn as `shown`, for trying a change out without making it. `scene` is what's given back in both forms.
  Preview { scene: Scene, shown: Scene },
}

/// Another scene from the content folder, drawn with the one being rendered to see how the two look together, such as
/// a game's scores over a transition shown behind them. It plays alongside from the same start, with its own default
/// variables, and only the scene being rendered has outlines and shapes in the reply. The two scenes' layers are stacked
/// by z, with its layers under the scene's on the same z.
#[derive(Clone, PartialEq, serde::Deserialize, TS)]
pub struct ReferenceScene {
  pub name: String,
}

/// How many points to sample a scene at where the machine doesn't say. A display is drawn at its own size, and only one
/// the machine doesn't have is drawn at the width and height.
#[derive(Clone, Copy, serde::Deserialize, TS)]
pub struct SceneSize {
  /// Points along the strip of `Sequence` light layers
  pub lights: usize,
  /// Points across a display the machine doesn't have, and the width of a light layer on a plane it doesn't have
  pub width: usize,
  /// Points down a display the machine doesn't have, and the height of a light layer on a plane it doesn't have
  pub height: usize,
}
