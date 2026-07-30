use std::any::Any;

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

#[allow(unused)]
pub struct Anonymous;
#[allow(unused)]
pub struct Action;
#[allow(unused)]
pub struct On;
#[allow(unused)]
pub struct Off;

#[derive(Debug, Clone, Copy)]
pub struct SystemSpawned(pub u64);
#[derive(Debug, Clone, Copy)]
pub struct SystemDespawned(pub u64);