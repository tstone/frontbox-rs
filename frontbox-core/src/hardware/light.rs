use crate::prelude::*;

/// A light source: anything that turns on, addressed by where it's wired ([`LightAddress`])
#[derive(Debug, Clone, Default, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Light {
  pub name: String,
  pub address: LightAddress,
  #[cfg_attr(feature = "ts", ts(type = "Array<string>"))]
  pub tags: Vec<Box<dyn Tag>>,
  #[cfg_attr(feature = "ts", ts(type = "[number, number, number] | null"))]
  pub location: Option<Vec3>,
  /// The color of a light that only shows one brightness, such as a lamp's bulb, for previewing it. `None` for an RGB
  /// LED, which shows whatever color it's set to.
  pub color: Option<Rgb>,
}

impl Light {
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

impl PartialEq for Light {
  fn eq(&self, other: &Self) -> bool {
    self.name == other.name
  }
}

impl Eq for Light {}
