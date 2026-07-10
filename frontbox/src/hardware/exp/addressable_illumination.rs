use std::any::TypeId;
use std::ops::{Deref, DerefMut};

use crate::hardware::*;

#[derive(Debug, Clone)]
pub struct AddressableIllumination {
  pub leds: Vec<AddressableLed>,
  pub source: Box<dyn Illumination>,
}

impl AddressableIllumination {
  pub fn is<T: Illumination + 'static>(&self) -> bool {
    self.source.as_any().is::<T>()
  }

  pub fn has_tag<T: Tag + 'static>(&self) -> bool {
    self
      .tags()
      .iter()
      .any(|tag| <dyn Tag>::as_any(tag.as_ref()).is::<T>())
  }

  pub(crate) fn has_typed_tag(&self, type_id: TypeId) -> bool {
    self
      .tags()
      .iter()
      .any(|tag| <dyn Tag>::as_any(tag.as_ref()).type_id() == type_id)
  }
}

impl Deref for AddressableIllumination {
  type Target = Box<dyn Illumination>;

  fn deref(&self) -> &Self::Target {
    &self.source
  }
}

impl DerefMut for AddressableIllumination {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.source
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AddressableLed {
  pub address: LedAddress,
  pub index: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LedAddress {
  pub address: u8,
  pub breakout: Option<u8>,
  pub port: u8,
}
