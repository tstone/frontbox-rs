use std::collections::HashSet;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use serde::Serialize;
use serde_json::Value;

use crate::store::Storable;

#[derive(Clone, Debug, Serialize)]
pub struct StorableHashSet<T: Serialize + Hash + Send + Sync + Clone + 'static> {
  inner: HashSet<T>,
  key: &'static str,
}

impl<T: Serialize + Hash + Send + Sync + Clone + 'static> StorableHashSet<T> {
  pub fn new(key: &'static str) -> Self {
    Self {
      inner: HashSet::new(),
      key,
    }
  }

  pub fn from_set(set: HashSet<T>, key: &'static str) -> Self {
    Self { inner: set, key }
  }
}

impl<T: Serialize + Eq + Hash + Send + Sync + Clone + 'static> StorableHashSet<T> {
  pub fn from_vec(vec: Vec<T>, key: &'static str) -> Self {
    Self {
      inner: vec.into_iter().collect(),
      key,
    }
  }
}

impl<T: Serialize + Hash + Send + Sync + Clone + 'static> Storable for StorableHashSet<T> {
  fn to_json(&self) -> Value {
    serde_json::to_value(&self.inner).unwrap_or(Value::Null)
  }

  fn key(&self) -> &str {
    self.key
  }
}

impl<T: Serialize + Hash + Send + Sync + Clone + 'static> Deref for StorableHashSet<T> {
  type Target = HashSet<T>;
  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<T: Serialize + Hash + Send + Sync + Clone + 'static> DerefMut for StorableHashSet<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}
