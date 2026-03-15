use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use serde::Serialize;
use serde_json::Value;

use crate::prelude::Storable;

#[derive(Debug, Serialize)]
pub struct StorableHashMap<
  K: Serialize + Eq + Hash + Send + Sync + 'static,
  V: Serialize + Send + Sync + 'static,
> {
  inner: HashMap<K, V>,
  key: &'static str,
}

impl<K: Serialize + Eq + Hash + Send + Sync + 'static, V: Serialize + Send + Sync + 'static>
  StorableHashMap<K, V>
{
  pub fn new(key: &'static str) -> Self {
    Self {
      inner: HashMap::new(),
      key,
    }
  }

  pub fn from_map(map: HashMap<K, V>, key: &'static str) -> Self {
    Self { inner: map, key }
  }
}

impl<K: Serialize + Eq + Hash + Send + Sync + 'static, V: Serialize + Send + Sync + 'static>
  Storable for StorableHashMap<K, V>
{
  fn to_json(&self) -> Value {
    serde_json::to_value(&self.inner).unwrap_or(Value::Null)
  }

  fn key(&self) -> &str {
    self.key
  }
}

impl<K: Serialize + Eq + Hash + Send + Sync + 'static, V: Serialize + Send + Sync + 'static> Deref
  for StorableHashMap<K, V>
{
  type Target = HashMap<K, V>;
  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<K: Serialize + Eq + Hash + Send + Sync + 'static, V: Serialize + Send + Sync + 'static>
  DerefMut for StorableHashMap<K, V>
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}
