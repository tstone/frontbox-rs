use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::prelude::*;
use serde_json::Value;

pub trait StorableType: Any + Storable + Default + Send + Sync + 'static {}
impl<T: Any + Storable + Default + Send + Sync + 'static> StorableType for T {}

#[derive(Debug)]
pub struct Store {
  internal: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Store {
  pub fn new() -> Self {
    Self {
      internal: HashMap::new(),
    }
  }

  pub fn clear(&mut self) {
    self.internal.clear();
  }

  pub fn get<T: StorableType>(&self) -> Option<&T> {
    self
      .internal
      .get(&TypeId::of::<T>())
      .and_then(|boxed| boxed.downcast_ref::<T>())
  }

  pub fn get_mut<T: StorableType>(&mut self) -> &mut T {
    let type_id = TypeId::of::<T>();

    if !self.internal.contains_key(&type_id) {
      self.internal.insert(type_id, Box::new(T::default()));
    }

    self
      .internal
      .get_mut(&type_id)
      .unwrap()
      .downcast_mut::<T>()
      .unwrap()
  }

  pub fn insert<T: StorableType>(&mut self, value: T) {
    self.internal.insert(TypeId::of::<T>(), Box::new(value));
  }

  pub fn remove<T: StorableType>(&mut self) {
    self.internal.remove(&TypeId::of::<T>());
  }

  pub fn to_json(&self) -> Value {
    let mut map = serde_json::Map::new();

    for boxed in self.internal.values() {
      // Downcast to &dyn Storable
      if let Some(storable) = boxed.downcast_ref::<Box<dyn Storable>>() {
        map.insert(storable.key().to_string(), storable.to_json());
      }
    }

    serde_json::Value::Object(map)
  }
}
