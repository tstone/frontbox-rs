use std::any::TypeId;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct LED {
  pub name: String,
  pub address: LedAddress,
  pub tags: Vec<Box<dyn Tag>>,
  pub location: Option<Vec3>,
}

impl LED {
  pub fn has_tag<T: Tag + 'static>(&self) -> bool {
    self
      .tags
      .iter()
      .any(|tag| <dyn Tag>::as_any(tag.as_ref()).is::<T>())
  }

  pub(crate) fn has_typed_tag(&self, type_id: TypeId) -> bool {
    self
      .tags
      .iter()
      .any(|tag| <dyn Tag>::as_any(tag.as_ref()).type_id() == type_id)
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LedAddress {
  pub board: u8,
  pub breakout: Option<u8>,
  pub port: u8,
  pub index: u16,
}
