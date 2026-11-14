use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use crate::scene::{Font, Picture};

static NONE: LazyLock<Assets> = LazyLock::new(Assets::default);

/// Decoded images and loaded fonts by name, passed to sampling through
/// [`LayerFrame::assets`](crate::scene::LayerFrame::assets)
///
/// Scenes refer to images and fonts by name only. [`Content::assets_for`](crate::scene::Content::assets_for) builds this
/// for a scene. Clones share pixel and font data.
#[derive(Debug, Clone, Default)]
pub struct Assets {
  pictures: HashMap<String, Arc<Picture>>,
  fonts: HashMap<String, Font>,
}

impl Assets {
  pub fn new() -> Self {
    Self::default()
  }

  /// An empty set, for scenes with no images or text
  pub fn none() -> &'static Self {
    &NONE
  }

  pub fn insert_picture(&mut self, name: impl Into<String>, picture: impl Into<Arc<Picture>>) {
    self.pictures.insert(name.into(), picture.into());
  }

  pub fn insert_font(&mut self, name: impl Into<String>, font: Font) {
    self.fonts.insert(name.into(), font);
  }

  pub fn picture(&self, name: &str) -> Option<&Picture> {
    self.pictures.get(name).map(Arc::as_ref)
  }

  pub fn font(&self, name: &str) -> Option<&Font> {
    self.fonts.get(name)
  }
}
