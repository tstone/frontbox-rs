use std::any::TypeId;
use std::collections::{HashMap, HashSet};

use tokio::sync::mpsc;

use crate::prelude::*;

struct Listener {
  callback: Box<dyn Fn(&dyn FrontboxEvent, &mut Context) + Send>,
  id: u64,
}

pub(crate) struct Dispatcher {
  listeners: HashMap<TypeId, Vec<Listener>>,
  listener_district_map: HashMap<u64, &'static str>,
}

impl Dispatcher {
  pub fn new() -> Self {
    Self {
      listeners: HashMap::new(),
      listener_district_map: HashMap::new(),
    }
  }

  pub fn subscribe(
    &mut self,
    event_type: TypeId,
    system_id: u64,
    district_key: &'static str,
    callback: Box<dyn Fn(&dyn FrontboxEvent, &mut Context) + Send>,
  ) {
    let listener = Listener {
      callback,
      id: system_id,
    };
    self.listeners.entry(event_type).or_default().push(listener);
    self.listener_district_map.insert(system_id, district_key);
  }

  pub fn unsubscribe(&mut self, event_type: TypeId, system_id: u64) {
    if let Some(listeners) = self.listeners.get_mut(&event_type) {
      listeners.retain(|listener| listener.id != system_id);
    }
    self.listener_district_map.remove(&system_id);
  }

  pub fn emit(
    &self,
    listeners: HashSet<u64>,
    event: &dyn FrontboxEvent,
    sender: mpsc::UnboundedSender<MachineCommand>,
    store: &HashMap<&'static str, Box<dyn StorageDistrict>>,
    switches: &SwitchContext,
    game_state: &Option<GameState>,
    config: &MachineConfig,
  ) {
    let event_type = event.type_id();
    if let Some(all_listeners) = self.listeners.get(&event_type) {
      for listener in all_listeners {
        if listeners.contains(&listener.id) {
          let mut ctx = Context::new(
            sender.clone(),
            store,
            switches,
            game_state,
            config,
            listener.id,
            self.listener_district_map.get(&listener.id).unwrap(),
          );
          (listener.callback)(event, &mut ctx);
        }
      }
    }
  }

  pub fn remove_district_listeners(&mut self, district_key: &'static str) {
    let system_ids_to_remove: Vec<u64> = self
      .listener_district_map
      .iter()
      .filter_map(|(&system_id, &key)| {
        if key == district_key {
          Some(system_id)
        } else {
          None
        }
      })
      .collect();

    for system_id in system_ids_to_remove {
      self.listener_district_map.remove(&system_id);
      for listeners in self.listeners.values_mut() {
        listeners.retain(|listener| listener.id != system_id);
      }
    }
  }
}
