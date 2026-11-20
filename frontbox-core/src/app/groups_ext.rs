use std::any::TypeId;
use std::cell::RefMut;

use crate::app::{Groups, ROOT_GROUP};
use crate::prelude::*;
use crate::systems::GroupId;

/// Finding systems in the tree. Each system's children are kept in a group it owns, and the top-level systems in the
/// root's.
pub(crate) trait GroupsExt {
  /// The instance of the type nearest to `from`: among its children, then its siblings, then its parent's siblings,
  /// and so on up to the top level
  fn nearest(&self, from: SystemHandle, type_id: TypeId) -> Option<SystemHandle>;

  /// The instance of the type among the system's children
  fn child_of_type(&self, parent: SystemHandle, type_id: TypeId) -> Option<SystemHandle>;

  /// Every instance of the type, in the order events are delivered
  fn instances(&self, type_id: TypeId) -> Vec<SystemHandle>;

  /// The group holding the system's children, if it has had any
  fn children_group(&self, owner: SystemHandle) -> Option<GroupId>;

  /// The system's parent, or `None` for a top-level system
  fn parent(&self, handle: SystemHandle) -> Option<SystemHandle>;

  /// The handle of the system with this id, wherever it is
  fn find(&self, id: u64) -> Option<SystemHandle>;

  fn system(&self, handle: SystemHandle) -> Option<RefMut<'_, SystemContainer>>;

  fn contains_system(&self, handle: SystemHandle) -> bool;

  fn system_name(&self, handle: SystemHandle) -> Option<&'static str>;

  /// Handles for every system in the group, in spawn order. Empty if there's no such group.
  fn handles_in(&self, group: GroupId) -> Vec<SystemHandle>;
}

impl GroupsExt for Groups {
  fn nearest(&self, from: SystemHandle, type_id: TypeId) -> Option<SystemHandle> {
    let search = |key: GroupId| {
      self
        .get(&key)
        .and_then(|group| group.id_by_type_id(&type_id))
        .map(|id| SystemHandle::new(id, key))
    };

    if let Some(found) = self.children_group(from).and_then(search) {
      return Some(found);
    }
    let mut key = from.group;
    loop {
      if let Some(found) = search(key) {
        return Some(found);
      }
      key = self.get(&key)?.owner?.group;
    }
  }

  fn child_of_type(&self, parent: SystemHandle, type_id: TypeId) -> Option<SystemHandle> {
    let key = self.children_group(parent)?;
    let id = self.get(&key)?.id_by_type_id(&type_id)?;
    Some(SystemHandle::new(id, key))
  }

  fn instances(&self, type_id: TypeId) -> Vec<SystemHandle> {
    self
      .iter()
      .filter_map(|(key, group)| {
        group
          .id_by_type_id(&type_id)
          .map(|id| SystemHandle::new(id, *key))
      })
      .collect()
  }

  fn children_group(&self, owner: SystemHandle) -> Option<GroupId> {
    self
      .iter()
      .find(|(key, group)| **key != ROOT_GROUP && group.owner == Some(owner))
      .map(|(key, _)| *key)
  }

  fn parent(&self, handle: SystemHandle) -> Option<SystemHandle> {
    self.get(&handle.group)?.owner
  }

  fn find(&self, id: u64) -> Option<SystemHandle> {
    self
      .iter()
      .find(|(_, group)| group.contains_id(&id))
      .map(|(key, _)| SystemHandle::new(id, *key))
  }

  fn system(&self, handle: SystemHandle) -> Option<RefMut<'_, SystemContainer>> {
    self
      .get(&handle.group)
      .and_then(|group| group.get_by_id(&handle.id))
  }

  fn contains_system(&self, handle: SystemHandle) -> bool {
    self
      .get(&handle.group)
      .is_some_and(|group| group.contains_id(&handle.id))
  }

  fn system_name(&self, handle: SystemHandle) -> Option<&'static str> {
    self
      .get(&handle.group)
      .and_then(|group| group.name_of(&handle.id))
  }

  fn handles_in(&self, group: GroupId) -> Vec<SystemHandle> {
    self
      .get(&group)
      .map(|g| {
        g.ids()
          .into_iter()
          .map(|id| SystemHandle::new(*id, group))
          .collect()
      })
      .unwrap_or_default()
  }
}
