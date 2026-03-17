use std::any::{TypeId, type_name};
use std::collections::HashMap;

#[derive(Default, Clone, Debug)]
pub struct TypeNameCache {
  map: HashMap<TypeId, &'static str>,
}

impl TypeNameCache {
  pub fn insert<T: 'static>(&mut self) -> TypeId {
    let id = TypeId::of::<T>();
    let name = type_name::<T>();
    self.map.insert(id, name);
    id
  }

  pub fn get_name(&self, id: TypeId) -> Option<&'static str> {
    self.map.get(&id).copied()
  }
}