use std::any::TypeId;

use crate::app::event_box::EventBox;
use crate::app::{Groups, GroupsExt};
use crate::prelude::*;
use crate::systems::state::StoredState;

// States are machine-wide, so they're all kept in the root's map, whichever system set them.

/// The value of `S`, or the default if it hasn't been set
pub(crate) fn state<S: State>(groups: &Groups) -> S {
  groups
    .get(&ROOT_GROUP)
    .and_then(|root| root.states.get(&TypeId::of::<S>()))
    .and_then(|stored| stored.value.downcast_ref::<S>().cloned())
    .unwrap_or_default()
}

/// Stores a state, returning the `StateChanged` to broadcast if its value changed. Ignored if another system owns it,
/// or if the owner has been despawned since setting it.
pub(crate) fn set_state(groups: &mut Groups, state: StoredState) -> Option<EventBox> {
  if !groups.contains_system(state.owner) {
    log::warn!(
      target: "frontbox::systems",
      "System {} set {} after it was despawned, ignored",
      state.owner.id,
      state.name
    );
    return None;
  }

  let states = &mut groups.get_mut(&ROOT_GROUP)?.states;
  match states.get_mut(&state.type_id) {
    Some(stored) if stored.owner != state.owner => {
      log::warn!(
        target: "frontbox::systems",
        "{} belongs to system {}, so system {} can't set it, ignored",
        state.name,
        stored.owner.id,
        state.owner.id
      );
      None
    }
    Some(stored) => {
      let changed = (state.into_changed)(Some(stored.value.as_ref()), Some(state.value.as_ref()));
      stored.value = state.value;
      changed
    }
    None => {
      let changed = (state.into_changed)(None, Some(state.value.as_ref()));
      states.insert(state.type_id, state);
      changed
    }
  }
}

/// Drops the states the system owns, returning them. See [`StoredState::changed_to_default`] for what to broadcast.
pub(crate) fn clear_states_owned_by(groups: &mut Groups, owner: SystemHandle) -> Vec<StoredState> {
  let Some(root) = groups.get_mut(&ROOT_GROUP) else {
    return Vec::new();
  };

  let owned: Vec<TypeId> = root
    .states
    .iter()
    .filter(|(_, state)| state.owner == owner)
    .map(|(type_id, _)| *type_id)
    .collect();

  owned
    .into_iter()
    .filter_map(|type_id| root.states.remove(&type_id))
    .collect()
}
