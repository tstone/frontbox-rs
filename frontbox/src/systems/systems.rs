use std::any::TypeId;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;

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

  pub(crate) fn insert(&mut self, system: impl Into<SystemContainer>) {
    let system = system.into();
    self.type_to_id.insert(system.type_id(), system.id());
    self.id_to_type.insert(system.id(), system.type_id());
    self.systems.insert(system.id(), RefCell::new(system));
  }

  pub(crate) fn remove(&mut self, system_id: u64) -> Option<RefCell<SystemContainer>> {
    let result = self.systems.remove(&system_id);
    if let Some(system_type) = self.id_to_type.remove(&system_id) {
      self.type_to_id.remove(&system_type);
    }
    result
  }

  pub fn get_by_id(&self, system_id: u64) -> Option<&RefCell<SystemContainer>> {
    self.systems.get(&system_id)
  }

  pub fn get_mut_by_id(&'_ mut self, system_id: u64) -> Option<RefMut<'_, SystemContainer>> {
    self.systems.get(&system_id).map(|cell| cell.borrow_mut())
  }

  pub fn get<T: System + 'static>(&'_ self) -> Option<Ref<'_, T>> {
    let type_id = TypeId::of::<T>();
    let system_id = self.type_to_id.get(&type_id)?;

    self.systems.get(system_id).map(|cell| {
      Ref::map(cell.borrow(), |container| {
        container
          .downcast_ref::<T>()
          .expect("type_to_id mapping was incorrect")
      })
    })
  }

  pub fn get_mut<T: System + 'static>(&'_ self) -> Option<RefMut<'_, T>> {
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

  pub fn contains_id(&self, system_id: u64) -> bool {
    self.systems.contains_key(&system_id)
  }

  pub(crate) fn values(&self) -> impl Iterator<Item = &RefCell<SystemContainer>> {
    self.systems.values()
  }

  pub(crate) fn values_mut(&'_ mut self) -> impl Iterator<Item = RefMut<'_, SystemContainer>> + '_ {
    self.systems.values().map(|cell| cell.borrow_mut())
  }
}
