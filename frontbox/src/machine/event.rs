use std::any::Any;
use std::fmt::Debug;
use std::sync::atomic::AtomicU64;

use crate::prelude::*;

static LISTENER_ID: AtomicU64 = AtomicU64::new(0);

pub(crate) fn next_listener_id() -> u64 {
  LISTENER_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

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
