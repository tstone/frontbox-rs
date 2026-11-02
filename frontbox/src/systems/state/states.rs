use std::any::TypeId;

use crate::app::event_box::EventBox;
use crate::app::{Groups, GroupsExt};
use crate::prelude::*;
use crate::systems::state::StoredState;

/// The value of `S` nearest to `from`: as set in the group itself, then the root. The default if neither has set it.
pub(crate) fn nearest_state<S: State>(groups: &Groups, from: GroupId) -> S {
  let search = |key: GroupId| {
    groups
      .get(&key)?
      .states
      .get(&TypeId::of::<S>())?
      .value
      .downcast_ref::<S>()
      .cloned()
  };

  search(from)
    .or_else(|| {
      if from != ROOT_GROUP {
        search(ROOT_GROUP)
      } else {
        None
      }
    })
    .unwrap_or_default()
}

/// Stores a state in its owner's group, returning the `StateChanged` to broadcast if its value changed. Ignored if
/// another system in the group owns it, or if the owner has been despawned since setting it.
pub(crate) fn set_state(groups: &mut Groups, state: StoredState) -> Option<EventBox> {
  if !groups.contains_system(state.owner) {
    log::warn!(
      "System {} set {} after it was despawned, ignored",
      state.owner.id,
      state.name
    );
    return None;
  }

  let group = groups.get_mut(&state.owner.group)?;
  match group.states.get_mut(&state.type_id) {
    Some(stored) if stored.owner != state.owner => {
      log::warn!(
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
      group.states.insert(state.type_id, state);
      changed
    }
  }
}

/// Drops the states the system owns, returning the `StateChanged` back to the default for each that wasn't already
pub(crate) fn clear_states_owned_by(groups: &mut Groups, owner: SystemHandle) -> Vec<EventBox> {
  let Some(group) = groups.get_mut(&owner.group) else {
    return Vec::new();
  };

  let owned: Vec<TypeId> = group
    .states
    .iter()
    .filter(|(_, state)| state.owner == owner)
    .map(|(type_id, _)| *type_id)
    .collect();

  owned
    .into_iter()
    .filter_map(|type_id| group.states.remove(&type_id))
    .filter_map(|state| (state.into_changed)(Some(state.value.as_ref()), None))
    .collect()
}
