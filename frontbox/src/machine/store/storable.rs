use std::any::Any;

pub trait Storable: Any + Send + Sync {
  fn to_json(&self) -> serde_json::Value;
  fn key(&self) -> &str;
}
