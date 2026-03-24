use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::store::type_name_cache::TypeNameCache;
use crate::store::{Storable, StorableType};
use serde_json::Value;

#[derive(Debug)]
pub struct Store {
  internal: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
  name_cache: TypeNameCache,
}

impl Store {
  pub fn new() -> Self {
    Self {
      internal: HashMap::new(),
      name_cache: TypeNameCache::default(),
    }
  }

  pub fn clear(&mut self) {
    log::warn!("Clearing Store");
    self.internal.clear();
  }

  /// Returns true if the store contains a value of type T
  pub fn has<T: StorableType>(&self) -> bool {
    self.internal.contains_key(&TypeId::of::<T>())
  }

  pub fn get<T: StorableType>(&self) -> Option<&T> {
    self
      .internal
      .get(&TypeId::of::<T>())
      .and_then(|boxed| boxed.downcast_ref::<T>())
  }

  pub fn cloned<T: StorableType + Clone>(&self) -> Option<T> {
    self.get::<T>().cloned()
  }

  /// Get the value of type T from the store, or panic if it doesn't exist
  pub fn expect<T: StorableType>(&self) -> &T {
    if let Some(value) = self.get::<T>() {
      value
    } else {
      self.trace_contents();
      panic!(
        "Expected {} ({:?}) value not found in Store",
        std::any::type_name::<T>(),
        TypeId::of::<T>()
      );
    }
  }

  pub fn get_or_default<T: StorableType + Default>(&self) -> &T {
    if !self.internal.contains_key(&TypeId::of::<T>()) {
      log::trace!(
        "{} not found in Store, returning default value",
        std::any::type_name::<T>()
      );
      T::default();
    }
    self.get::<T>().unwrap()
  }

  pub fn get_mut<T: StorableType>(&mut self) -> Option<&mut T> {
    self
      .internal
      .get_mut(&TypeId::of::<T>())
      .and_then(|boxed| boxed.downcast_mut::<T>())
  }

  /// Get a mutable reference to the value of type T from the store, or panic if it doesn't exist
  pub fn expect_mut<T: StorableType>(&mut self) -> &mut T {
    if self.internal.contains_key(&TypeId::of::<T>()) {
      self.get_mut::<T>().unwrap()
    } else {
      self.trace_contents();
      panic!(
        "Expected {} ({:?}) value not found in Store",
        std::any::type_name::<T>(),
        TypeId::of::<T>()
      );
    }
  }

  pub fn get_or_insert<T: StorableType + Default>(&mut self) -> &mut T {
    if !self.internal.contains_key(&TypeId::of::<T>()) {
      self.insert(T::default());
    }
    self.get_mut::<T>().unwrap()
  }

  pub fn insert<T: StorableType>(&mut self, value: T) {
    log::trace!("Inserting {} into Store", std::any::type_name::<T>());
    self.name_cache.insert::<T>();
    self.internal.insert(TypeId::of::<T>(), Box::new(value));
  }

  pub fn remove<T: StorableType>(&mut self) {
    log::trace!("Removing {} from Store", std::any::type_name::<T>());
    self.internal.remove(&TypeId::of::<T>());
  }

  pub fn trace_contents(&self) {
    log::trace!("Store contents:");
    for type_id in self.internal.keys() {
      let type_name = self.name_cache.get_name(*type_id).unwrap_or("Unknown");
      log::trace!("  {} ({:?})", type_name, type_id);
    }
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
