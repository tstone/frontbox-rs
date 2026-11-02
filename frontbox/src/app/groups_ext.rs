use std::any::TypeId;
use std::cell::RefMut;

use crate::app::Groups;
use crate::prelude::*;

/// Lookups across system groups
pub(crate) trait GroupsExt {
  /// The instance of the type nearest to the group: in the group itself, then the root. Nearest wins, so an instance
  /// in a player's group takes priority over one at the root.
  fn nearest(&self, from: &'static str, type_id: TypeId) -> Option<SystemHandle>;

  fn system(&self, handle: SystemHandle) -> Option<RefMut<'_, SystemContainer>>;

  fn contains_system(&self, handle: SystemHandle) -> bool;

  fn system_name(&self, handle: SystemHandle) -> Option<&'static str>;

  /// Handles for every system in the group, in spawn order. Empty if there's no such group.
  fn handles_in(&self, group: &'static str) -> Vec<SystemHandle>;
}

impl GroupsExt for Groups {
  fn nearest(&self, from: &'static str, type_id: TypeId) -> Option<SystemHandle> {
    let search = |key: &'static str| {
      self
        .get(key)
        .and_then(|group| group.id_by_type_id(&type_id))
        .map(|id| SystemHandle::new(id, key))
    };

    search(from).or_else(|| {
      if from != ROOT_GROUP {
        search(ROOT_GROUP)
      } else {
        None
      }
    })
  }

  fn system(&self, handle: SystemHandle) -> Option<RefMut<'_, SystemContainer>> {
    self
      .get(handle.parent_key)
      .and_then(|group| group.get_by_id(&handle.id))
  }

  fn contains_system(&self, handle: SystemHandle) -> bool {
    self
      .get(handle.parent_key)
      .is_some_and(|group| group.contains_id(&handle.id))
  }

  fn system_name(&self, handle: SystemHandle) -> Option<&'static str> {
    self.system(handle).map(|system| system.name())
  }

  fn handles_in(&self, group: &'static str) -> Vec<SystemHandle> {
    self
      .get(group)
      .map(|g| {
        g.ids()
          .into_iter()
          .map(|id| SystemHandle::new(*id, group))
          .collect()
      })
      .unwrap_or_default()
  }
}
