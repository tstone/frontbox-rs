use std::any::Any;

use crate::systems::SystemHandle;

pub trait Event: Any + Send + Sync {
  fn as_any(&self) -> &dyn Any;
}

impl<T: Any + Send + Sync> Event for T {
  fn as_any(&self) -> &dyn Any {
    self
  }
}

pub trait EventExt {
  fn is<T: Any>(&self) -> bool;
  fn downcast_ref<T: Any>(&self) -> Option<&T>;
}

impl EventExt for dyn Event {
  fn is<T: Any>(&self) -> bool {
    self.as_any().is::<T>()
  }

  fn downcast_ref<T: Any>(&self) -> Option<&T> {
    self.as_any().downcast_ref::<T>()
  }
}

/// Event that happens when a new system is spawned
#[derive(Debug, Clone, Copy)]
pub struct SystemSpawned(pub SystemHandle);

/// Event that happens when an existing system is despawned
#[derive(Debug, Clone, Copy)]
pub struct SystemDespawned(pub SystemHandle);