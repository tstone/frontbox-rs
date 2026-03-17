use std::any::Any;

pub trait Command: Any + Send + Sync {
  fn as_any(&self) -> &dyn Any;
}

impl<T: Any + Send + Sync> Command for T {
  fn as_any(&self) -> &dyn Any {
    self
  }
}

pub trait CommandExt {
  fn is<T: Any>(&self) -> bool;
  fn downcast_ref<T: Any>(&self) -> Option<&T>;
}

impl CommandExt for dyn Command {
  fn is<T: Any>(&self) -> bool {
    self.as_any().is::<T>()
  }

  fn downcast_ref<T: Any>(&self) -> Option<&T> {
    self.as_any().downcast_ref::<T>()
  }
}
