use std::any::TypeId;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::collections::hash_map::Keys;
use std::fmt::Debug;

use crate::prelude::*;

pub struct Systems {
  systems: HashMap<u64, RefCell<SystemContainer>>,
  type_to_id: HashMap<TypeId, u64>,
  id_to_type: HashMap<u64, TypeId>,
}

impl Systems {
  pub fn new() -> Self {
    Self {
      systems: HashMap::new(),
      type_to_id: HashMap::new(),
      id_to_type: HashMap::new(),
    }
  }

  pub fn ids(&self) -> Keys<'_, u64, RefCell<SystemContainer>> {
    self.systems.keys()
  }

  pub(crate) fn insert(&mut self, system: impl Into<SystemContainer>) {
    let system = system.into();
    self.type_to_id.insert(system.type_id(), system.id());
    self.id_to_type.insert(system.id(), system.type_id());
    self.systems.insert(system.id(), RefCell::new(system));
  }

  pub(crate) fn reinsert(&mut self, system_id: u64, cell: RefCell<SystemContainer>) {
    self.systems.insert(system_id, cell);
  }

  pub(crate) fn remove(&mut self, system_id: u64) -> Option<RefCell<SystemContainer>> {
    let result = self.systems.remove(&system_id);
    if let Some(system_type) = self.id_to_type.remove(&system_id) {
      self.type_to_id.remove(&system_type);
    }
    result
  }

  /// Lease removes the system from the systems collection and returns it, but does not remove the type/id mapping.
  /// This is used for temporarily taking ownership of a system to spawn it as a child. Reinsert with `reinsert` when done.
  pub(crate) fn lease(&mut self, system_id: u64) -> Option<RefCell<SystemContainer>> {
    self.systems.remove(&system_id)
  }

  pub fn get_id<T: System + 'static>(&self) -> Option<u64> {
    let type_id = TypeId::of::<T>();
    self.type_to_id.get(&type_id).copied()
  }

  pub fn get_by_id(&'_ self, system_id: &u64) -> Option<RefMut<'_, SystemContainer>> {
    self.systems.get(system_id).map(|cell| cell.borrow_mut())
  }

  pub fn get_by_type<T: System + 'static>(&'_ self) -> Option<RefMut<'_, T>> {
    let type_id = TypeId::of::<T>();
    let system_id = self.type_to_id.get(&type_id)?;
    self.systems.get(system_id).map(|cell| {
      RefMut::map(cell.borrow_mut(), |container| {
        container
          .downcast_mut::<T>()
          .expect("type_to_id mapping was incorrect")
      })
    })
  }

  pub fn contains<T: System + 'static>(&self) -> bool {
    let type_id = TypeId::of::<T>();
    self.type_to_id.contains_key(&type_id)
  }

  pub fn contains_id(&self, system_id: &u64) -> bool {
    self.systems.contains_key(system_id)
  }

  pub(crate) fn values(&self) -> impl Iterator<Item = &RefCell<SystemContainer>> {
    self.systems.values()
  }

  pub(crate) fn values_mut(&'_ mut self) -> impl Iterator<Item = RefMut<'_, SystemContainer>> + '_ {
    self.systems.values().map(|cell| cell.borrow_mut())
  }
}

impl Debug for Systems {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Systems")
      .field(
        "systems",
        &self
          .systems
          .iter()
          .map(|(id, cell)| (id, cell.borrow().name().to_string()))
          .collect::<HashMap<_, _>>(),
      )
      .finish()
  }
}
