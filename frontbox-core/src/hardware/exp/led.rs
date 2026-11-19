
use crate::prelude::*;

#[derive(Debug, Clone, Default, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LED {
  pub name: String,
  pub address: LightAddress,
  #[cfg_attr(feature = "ts", ts(type = "Array<string>"))]
  pub tags: Vec<Box<dyn Tag>>,
  #[cfg_attr(feature = "ts", ts(type = "[number, number, number] | null"))]
  pub location: Option<Vec3>,
}

impl LED {
  pub fn has_tag<T: Tag + 'static>(&self) -> bool {
    self
      .tags
      .iter()
      .any(|tag| <dyn Tag>::as_any(tag.as_ref()).is::<T>())
  }

  pub(crate) fn has_tag_named(&self, name: &str) -> bool {
    self.tags.iter().any(|tag| tag.tag_name() == name)
  }
}

impl PartialEq for LED {
  fn eq(&self, other: &Self) -> bool {
    self.name == other.name
  }
}

impl Eq for LED {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LedAddress {
  pub exp: ExpAddress,
  /// The LED port on the board, 0-based. Only for display: `index` already counts from the start of the breakout.
  pub port: u8,
  pub index: u8,
}

impl LedAddress {
  pub fn new(exp: ExpAddress, port: u8, index: u8) -> Self {
    Self { exp, port, index }
  }

  pub fn board(&self) -> u8 {
    self.exp.board_address
  }

  pub fn breakout(&self) -> Option<u8> {
    self.exp.breakout
  }

}
