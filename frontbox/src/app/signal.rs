use std::any::Any;

pub trait Signal: Any + Send + Sync {
  fn as_any(&self) -> &dyn Any;
}

impl<T: Any + Send + Sync> Signal for T {
  fn as_any(&self) -> &dyn Any {
    self
  }
}

pub trait SignalExt {
  fn is<T: Any>(&self) -> bool;
  fn downcast_ref<T: Any>(&self) -> Option<&T>;
}

impl SignalExt for dyn Signal {
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
pub struct Named(&'static str);
#[allow(unused)]
pub struct On(&'static str);
#[allow(unused)]
pub struct Off(&'static str);
