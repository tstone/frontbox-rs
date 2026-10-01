use axum::extract::ws::Utf8Bytes;
use frontbox::prelude::{Hardware, SystemDespawned, SystemSpawned};
use frontbox::prelude::app_tracer::TraceEvent;
use frontbox_turn_based::{GameEnded, GameStarted, PlayerTurnBeginning};
use std::any::type_name;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::broadcast;

use crate::console_plane::ConsolePlane;
use crate::protocol::*;

const LOG_CAPACITY: usize = 1000;
/// How many emitted events each system keeps for the systems view
const RECENT_EVENTS_PER_SYSTEM: usize = 10;
const BROADCAST_CAPACITY: usize = 1024;

/// Singleton tracking data coming out of AppTracer, providing to multiple console connection
#[derive(Clone)]
pub(crate) struct ConsoleHub {
  state: Arc<Mutex<HubState>>,
  tx: broadcast::Sender<Utf8Bytes>,
}

struct HubState {
  snapshot: Snapshot,
  started: Instant,
  next_seq: u64,
  /// Image files for `snapshot.planes`, by the same index
  plane_images: Vec<Option<PathBuf>>,
}

impl ConsoleHub {
  pub fn new() -> Self {
    let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
    Self {
      state: Arc::new(Mutex::new(HubState {
        snapshot: Snapshot::default(),
        started: Instant::now(),
        next_seq: 0,
        plane_images: Vec::new(),
      })),
      tx,
    }
  }

  /// Returns the current `Init` message along with a receiver for everything after it
  pub fn subscribe(&self) -> Option<(Utf8Bytes, broadcast::Receiver<Utf8Bytes>)> {
    let state = self.state.lock().unwrap();
    let init = serialize(&ServerMessage::Init(state.snapshot.clone()))?;
    Some((init, self.tx.subscribe()))
  }

  /// Planes come from the tracer's configuration, before any client connects
  pub fn set_planes(&self, planes: &[ConsolePlane]) {
    let mut state = self.state.lock().unwrap();
    state.snapshot.planes = planes
      .iter()
      .enumerate()
      .map(|(index, console_plane)| {
        let (origin, rotation) = console_plane.plane.world_transform();
        PlaneView {
          name: console_plane.name.clone(),
          origin: origin.to_array(),
          rotation: rotation.to_array(),
          extent: console_plane.plane.extent.to_array(),
          image: console_plane
            .image
            .as_ref()
            .map(|_| format!("/planes/{index}/image")),
          code: console_plane.code.clone(),
        }
      })
      .collect();
    state.plane_images = planes.iter().map(|p| p.image.clone()).collect();
  }

  pub fn plane_image(&self, index: usize) -> Option<PathBuf> {
    let state = self.state.lock().unwrap();
    state.plane_images.get(index).cloned().flatten()
  }

  pub fn set_hardware(&self, hardware: Hardware) {
    let mut state = self.state.lock().unwrap();
    state.snapshot.hardware = Some(hardware);
    // hardware only arrives once at boot, so just have clients start over
    if let Some(init) = serialize(&ServerMessage::Init(state.snapshot.clone())) {
      let _ = self.tx.send(init);
    }
  }

  pub fn ingest(&self, event: TraceEvent) {
    if is_redundant(&event) {
      return;
    }

    let mut state = self.state.lock().unwrap();
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
    TraceEvent::Event { type_name: name, .. } => {
      *name == type_name::<SystemSpawned>() || *name == type_name::<SystemDespawned>()
    }
    _ => false,
  }
}

/// Changes to the game in progress, recognized from `frontbox-turn-based` events
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
      log::error!(target: "frontbox_pin_console", "Unable to serialize console message: {err}");
      None
    }
  }
}
