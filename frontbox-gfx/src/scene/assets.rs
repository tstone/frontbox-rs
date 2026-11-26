use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use crate::scene::{EnvelopeKey, Font, LoadedGenerator, Picture, PreparedEnvelope};

static NONE: LazyLock<Assets> = LazyLock::new(Assets::default);

/// Decoded images, loaded fonts, and shape generators by name, and prepared external envelopes, passed to sampling through
/// [`LayerFrame::assets`](crate::scene::LayerFrame::assets)
///
/// Scenes refer to images and fonts by name only. [`Content::assets_for`](crate::scene::Content::assets_for) builds this
/// for a scene's layers, and the scene adds the envelopes they follow, as only it knows where its sounds play. Clones
/// share pixel, font, and envelope data.
#[derive(Debug, Clone, Default)]
pub struct Assets {
  pictures: HashMap<String, Arc<Picture>>,
  fonts: HashMap<String, Font>,
  generators: HashMap<String, LoadedGenerator>,
  envelopes: HashMap<EnvelopeKey, Arc<PreparedEnvelope>>,
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

  pub fn insert_shape_generator(&mut self, name: impl Into<String>, generator: LoadedGenerator) {
    self.generators.insert(name.into(), generator);
  }

  pub fn insert_envelope(&mut self, key: EnvelopeKey, envelope: impl Into<Arc<PreparedEnvelope>>) {
    self.envelopes.insert(key, envelope.into());
  }

  pub fn picture(&self, name: &str) -> Option<&Picture> {
    self.pictures.get(name).map(Arc::as_ref)
  }

  pub fn font(&self, name: &str) -> Option<&Font> {
    self.fonts.get(name)
  }

  pub fn shape_generator(&self, name: &str) -> Option<&LoadedGenerator> {
    self.generators.get(name)
  }

  pub fn envelope(&self, key: &EnvelopeKey) -> Option<&PreparedEnvelope> {
    self.envelopes.get(key).map(Arc::as_ref)
  }
}
