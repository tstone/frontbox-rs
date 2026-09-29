use axum::extract::ws::Utf8Bytes;
use frontbox::prelude::Hardware;
use frontbox::prelude::app_tracer::TraceEvent;
use frontbox_turn_based::{GameEnded, GameStarted, PlayerTurnBeginning};
use std::any::type_name;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::broadcast;

use crate::protocol::*;

const LOG_CAPACITY: usize = 1000;
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
}

impl ConsoleHub {
  pub fn new() -> Self {
    let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
    Self {
      state: Arc::new(Mutex::new(HubState {
        snapshot: Snapshot::default(),
        started: Instant::now(),
        next_seq: 0,
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

  pub fn set_hardware(&self, hardware: Hardware) {
    let mut state = self.state.lock().unwrap();
    state.snapshot.hardware = Some(hardware);
    // hardware only arrives once at boot, so just have clients start over
    if let Some(init) = serialize(&ServerMessage::Init(state.snapshot.clone())) {
      let _ = self.tx.send(init);
    }
  }

  pub fn ingest(&self, event: TraceEvent) {
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
