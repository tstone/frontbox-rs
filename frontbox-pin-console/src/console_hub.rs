use axum::extract::ws::Utf8Bytes;
use frontbox::prelude::app_tracer::{Color, TraceEvent, TracerControlEvent};
use frontbox::prelude::{Hardware, SystemDespawned, SystemSpawned};
use frontbox_pinball::{GameEnded, GameStarted, PlayerTurnBeginning};
use std::any::type_name;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};

use crate::console_plane::ConsolePlane;
use crate::protocol::*;

const LOG_CAPACITY: usize = 2500;
const RECENT_EVENTS_PER_SYSTEM: usize = 10;

/// Messages buffered per client for events and other state. A client further behind than this gets everything again.
const BROADCAST_CAPACITY: usize = 1024;
/// LED updates buffered per client. LEDs are display-only and change constantly, so a client that falls behind skips
/// them (and is sent the current colors) rather than holding up anything else.
const LED_BROADCAST_CAPACITY: usize = 4;

#[derive(Clone)]
pub(crate) struct ConsoleHub {
  state: Arc<Mutex<HubState>>,
  tx: broadcast::Sender<Utf8Bytes>,
  leds_tx: broadcast::Sender<Utf8Bytes>,
}

impl ConsoleHub {
  pub fn new() -> Self {
    let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
    let (leds_tx, _) = broadcast::channel(LED_BROADCAST_CAPACITY);
    Self {
      state: Arc::new(Mutex::new(HubState {
        snapshot: Snapshot::default(),
        started: Instant::now(),
        next_seq: 0,
        plane_images: Vec::new(),
        control: None,
        led_names: HashMap::new(),
        pending_leds: BTreeMap::new(),
      })),
      tx,
      leds_tx,
    }
  }

  pub fn subscribe(&self) -> Option<Subscription> {
    let state = self.state.lock().unwrap();
    let init = serialize(&ServerMessage::Init(state.snapshot.clone()))?;
    Some(Subscription {
      init,
      events: self.tx.subscribe(),
      leds: self.leds_tx.subscribe(),
    })
  }

  pub fn all_leds(&self) -> Option<Utf8Bytes> {
    let state = self.state.lock().unwrap();
    serialize(&ServerMessage::Leds(LedColors {
      colors: state.snapshot.led_colors.clone(),
    }))
  }

  /// Called before any client connects, so nothing is broadcast
  pub fn set_planes(&self, planes: &[ConsolePlane]) {
    let mut state = self.state.lock().unwrap();
    state.snapshot.planes = planes
      .iter()
      .enumerate()
      .map(|(index, ConsolePlane { plane, code })| {
        let (origin, rotation) = plane.world_transform();
        PlaneView {
          name: plane.name.to_string(),
          origin: origin.to_array(),
          rotation: rotation.to_array(),
          extent: plane.extent.to_array(),
          image: plane
            .image
            .as_ref()
            .map(|_| format!("/planes/{index}/image")),
          code: code.map(str::to_string),
        }
      })
      .collect();
    state.plane_images = planes.iter().map(|p| p.plane.image.clone()).collect();
  }

  pub fn set_control(&self, control: mpsc::UnboundedSender<TracerControlEvent>) {
    self.state.lock().unwrap().control = Some(control);
  }

  /// Close or open a switch in the app, as if it were pressed on the machine. The change shows up for clients when the
  /// app traces it back, like any other switch change.
  pub fn set_switch(&self, switch_id: usize, closed: bool) {
    let state = self.state.lock().unwrap();
    let known = state
      .snapshot
      .hardware
      .as_ref()
      .is_some_and(|hw| hw.switches.by_id(&switch_id).is_some());
    if !known {
      log::warn!(target: "frontbox::console", "Ignoring a request to set unknown switch {switch_id}");
      return;
    }
    let Some(control) = &state.control else {
      return;
    };
    let _ = control.send(switch_control(switch_id, closed));
  }

  pub fn plane_image(&self, index: usize) -> Option<PathBuf> {
    let state = self.state.lock().unwrap();
    state.plane_images.get(index).cloned().flatten()
  }

  pub fn set_hardware(&self, hardware: Hardware) {
    let mut state = self.state.lock().unwrap();
    state.led_names = hardware
      .leds
      .values()
      .map(|led| {
        let exp = &led.address.exp;
        (
          (exp.board_address, exp.breakout, led.address.index),
          led.name.clone(),
        )
      })
      .collect();
    state.snapshot.hardware = Some(hardware);
    // hardware only arrives once at boot, so just have clients start over
    if let Some(init) = serialize(&ServerMessage::Init(state.snapshot.clone())) {
      let _ = self.tx.send(init);
    }
  }

  /// Called on a timer by the tracer
  pub fn flush_leds(&self) {
    let mut state = self.state.lock().unwrap();
    if state.pending_leds.is_empty() {
      return;
    }
    let colors = std::mem::take(&mut state.pending_leds);
    if let Some(message) = serialize(&ServerMessage::Leds(LedColors { colors })) {
      // no receivers is fine: LED updates are only for clients connected right now
      let _ = self.leds_tx.send(message);
    }
  }

  pub fn ingest(&self, event: TraceEvent) {
    if is_redundant(&event) {
      return;
    }

    let mut guard = self.state.lock().unwrap();
    // reborrow so fields of the state can be borrowed separately
    let state = &mut *guard;

    // LEDs change many times a second, in batches. They're current state, not history: keep the latest color and
    // let `flush_leds` send what changed at a steady rate, rather than logging and broadcasting every batch.
    if let TraceEvent::LedsRGBChange {
      expansion,
      breakout,
      states,
    } = &event
    {
      for (index, color) in states {
        if let Some(name) = state.led_names.get(&(*expansion, *breakout, *index)) {
          state.snapshot.led_colors.insert(name.clone(), *color);
          state.pending_leds.insert(name.clone(), *color);
        }
      }
      return;
    }

    let transition = GameTransition::from_event(&event);
    let game = &mut state.snapshot.game;
    match transition {
      Some(GameTransition::Started) => *game = Some(GameState::default()),
      Some(GameTransition::TurnBeginning { player, turn }) => {
        let game = game.get_or_insert_default();
        game.player = Some(player);
        game.turn = Some(turn);
      }
      Some(GameTransition::Ended) | None => {}
    }

    let record = TraceRecord {
      seq: state.next_seq,
      at_ms: state.started.elapsed().as_millis() as u64,
      // GameEnded is still tagged with the game it ended
      game: state.snapshot.game.clone(),
      event,
    };
    state.next_seq += 1;

    // Game starts and ends reset clients with a fresh `Init` so that game tracking only has to
    // live here. The console only shows the current game, so a start also drops the log.
    let message = match transition {
      Some(GameTransition::Started) => {
        state.snapshot.log.clear();
        apply(&mut state.snapshot, &record);
        ServerMessage::Init(state.snapshot.clone())
      }
      Some(GameTransition::Ended) => {
        apply(&mut state.snapshot, &record);
        state.snapshot.game = None;
        ServerMessage::Init(state.snapshot.clone())
      }
      _ => {
        apply(&mut state.snapshot, &record);
        ServerMessage::Trace(record)
      }
    };

    if let Some(message) = serialize(&message) {
      // no receivers is fine; nobody has the page open
      let _ = self.tx.send(message);
    }
  }
}

/// The framework also emits spawn/despawn events for systems to react to. The trace variants for
/// the same thing carry more (the system name), so the bus copies would only duplicate log rows.
fn is_redundant(event: &TraceEvent) -> bool {
  match event {
    TraceEvent::Event {
      type_name: name, ..
    } => *name == type_name::<SystemSpawned>() || *name == type_name::<SystemDespawned>(),
    _ => false,
  }
}

/// Changes to the game in progress, recognized from `frontbox-pinball` events
#[derive(Clone, Copy)]
enum GameTransition {
  Started,
  TurnBeginning { player: u8, turn: u8 },
  Ended,
}

impl GameTransition {
  fn from_event(event: &TraceEvent) -> Option<Self> {
    let TraceEvent::Event {
      type_name: name,
      event,
      ..
    } = event
    else {
      return None;
    };

    if *name == type_name::<GameStarted>() {
      Some(Self::Started)
    } else if *name == type_name::<GameEnded>() {
      Some(Self::Ended)
    } else if *name == type_name::<PlayerTurnBeginning>() {
      let field = |name: &str| {
        event
          .as_ref()
          .and_then(|body| body.get(name))
          .and_then(|value| value.as_u64())
          .map(|value| value as u8)
      };
      Some(Self::TurnBeginning {
        player: field("current_player")?,
        turn: field("turn")?,
      })
    } else {
      None
    }
  }
}

/// Mirror of the reducer in `web/src/state/console.ts`. Game tracking is not mirrored; see `ingest`.
fn apply(snapshot: &mut Snapshot, record: &TraceRecord) {
  let groups = &mut snapshot.groups;
  match &record.event {
    TraceEvent::SystemGroupSpawned { key } => {
      if !groups.iter().any(|g| g.key == *key) {
        groups.push(SystemGroup {
          key,
          active: true,
          systems: Vec::new(),
        });
      }
    }
    TraceEvent::SystemGroupDespawned { key } => groups.retain(|g| g.key != *key),
    TraceEvent::SystemGroupActiveStateChange { key, active } => {
      if let Some(group) = groups.iter_mut().find(|g| g.key == *key) {
        group.active = *active;
      }
    }
    TraceEvent::SystemSpawned {
      id,
      name,
      parent_key,
    } => {
      if let Some(group) = groups.iter_mut().find(|g| g.key == *parent_key) {
        group.systems.push(System {
          id: *id,
          name,
          active: true,
          recent_events: VecDeque::new(),
        });
      }
    }
    TraceEvent::SystemDespawned { id, parent_key } => {
      if let Some(group) = groups.iter_mut().find(|g| g.key == *parent_key) {
        group.systems.retain(|s| s.id != *id);
      }
    }
    TraceEvent::SystemActiveStateChange { id, active } => {
      if let Some(system) = groups
        .iter_mut()
        .flat_map(|g| g.systems.iter_mut())
        .find(|s| s.id == *id)
      {
        system.active = *active;
      }
    }
    TraceEvent::SwitchStateChange { switch_id, state } => {
      snapshot.switches.insert(*switch_id, state.clone());
    }
    TraceEvent::DriverStateChange { driver_id, state } => {
      snapshot.drivers.insert(*driver_id, state.clone());
    }
    TraceEvent::Event {
      sender: Some(sender),
      ..
    } => {
      if let Some(system) = groups
        .iter_mut()
        .flat_map(|g| g.systems.iter_mut())
        .find(|s| s.id == *sender)
      {
        if system.recent_events.len() >= RECENT_EVENTS_PER_SYSTEM {
          system.recent_events.pop_front();
        }
        system.recent_events.push_back(record.clone());
      }
    }
    TraceEvent::Event { .. } => {}
    // handled by `ingest`, never logged
    TraceEvent::LedsRGBChange { .. } => return,
  }

  if snapshot.log.len() >= LOG_CAPACITY {
    snapshot.log.pop_front();
  }
  snapshot.log.push_back(record.clone());
}

fn serialize(message: &ServerMessage) -> Option<Utf8Bytes> {
  match serde_json::to_string(message) {
    Ok(json) => Some(json.into()),
    Err(err) => {
      log::error!(target: "frontbox::console", "Unable to serialize console message: {err}");
      None
    }
  }
}

/// What a client gets when it first connects
pub(crate) struct Subscription {
  pub init: Utf8Bytes,
  pub events: broadcast::Receiver<Utf8Bytes>,
  pub leds: broadcast::Receiver<Utf8Bytes>,
}

struct HubState {
  snapshot: Snapshot,
  started: Instant,
  next_seq: u64,
  /// Image files for `snapshot.planes`, by the same index
  plane_images: Vec<Option<PathBuf>>,
  /// Where requests from clients (e.g. pressing a switch) go into the app. `None` until `AppTracer::init`.
  control: Option<mpsc::UnboundedSender<TracerControlEvent>>,
  led_names: LedNames,
  /// LED colors changed since the last `flush_leds`
  pending_leds: BTreeMap<String, Color>,
}

/// (expansion board address, breakout, index) to LED name
type LedNames = HashMap<(u8, Option<u8>, u16), String>;

/// The app's control event for setting a switch from the console
fn switch_control(switch_id: usize, closed: bool) -> TracerControlEvent {
  if closed {
    TracerControlEvent::CloseSwitch { switch_id }
  } else {
    TracerControlEvent::OpenSwitch { switch_id }
  }
}
