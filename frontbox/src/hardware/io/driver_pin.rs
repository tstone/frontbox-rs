use crate::{DriverMode, HardwareTag, NativeIdentity};

pub struct DriverPin(pub u16);

impl DriverPin {
  pub fn named(&self, name: &'static str) -> DriverMapping {
    DriverMapping {
      key: name,
      pin: self.0,
      tags: Vec::new(),
      mode: None,
    }
  }
}

/// Define a driver pin on an IO board
pub fn driver(index: u16) -> DriverPin {
  DriverPin(index)
}

pub struct DriverMapping {
  pub(crate) key: &'static str,
  pub(crate) pin: u16,
  pub(crate) tags: Vec<Box<dyn HardwareTag>>,
  pub(crate) mode: Option<Box<dyn DriverMode>>,
}

impl DriverMapping {
  pub fn tagged(mut self, tag: impl HardwareTag + 'static) -> Self {
    self.tags.push(Box::new(tag));
    self
  }

  pub fn mode(mut self, mode: impl DriverMode + 'static) -> Self {
    self.mode = Some(Box::new(mode));
    self
  }
}

#[derive(Clone)]
pub struct DriverDefinition {
  pub id: usize,
  pub name: &'static str,
  pub native: NativeIdentity,
  pub mode: Option<Box<dyn DriverMode>>,
  pub tags: Vec<Box<dyn HardwareTag>>,
}
