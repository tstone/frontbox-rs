use std::any::Any;
use std::collections::HashSet;

use serde::Serialize;
use serde_json::Value;

pub trait Storable: Any + Send + Sync {
  fn to_json(&self) -> serde_json::Value;
  fn key(&self) -> &str;
}

pub trait StorableType: Any + Serialize + Storable + Send + Sync {}
impl<T: Any + Storable + Serialize + Send + Sync> StorableType for T {}

impl<T: Storable> Storable for Vec<T> {
  fn to_json(&self) -> Value {
    Value::Array(self.iter().map(|item| item.to_json()).collect())
  }

  fn key(&self) -> &str {
    std::any::type_name::<T>()
  }
}

impl<T: Storable> Storable for HashSet<T> {
  fn to_json(&self) -> Value {
    Value::Array(self.iter().map(|item| item.to_json()).collect())
  }

  fn key(&self) -> &str {
    std::any::type_name::<T>()
  }
}
