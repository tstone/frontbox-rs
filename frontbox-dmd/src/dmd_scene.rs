use std::fmt;

use frontbox_gfx::scene::file::{self, FormatError};
use frontbox_gfx::scene::{GfxComposition, SceneError};
use serde::{Deserialize, Serialize};

/// What a DMD shows, as written to a `.scene` file. There's nothing to play one with yet.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmdScene {
  pub version: u32,
  pub composition: GfxComposition,
}

impl DmdScene {
  /// A scene of the current format version
  pub fn new(composition: GfxComposition) -> Self {
    Self { version: file::VERSION, composition }
  }

  /// Reads a `.scene` file, which has to be a DMD scene that can be played
  pub fn load(source: &str) -> Result<Self, FormatError> {
    let SceneFile::Dmd(scene) = file::load(source)?;
    file::check_version(scene.version)?;
    scene.check().map_err(|err| FormatError::Invalid(err.to_string()))?;
    Ok(scene)
  }

  /// Writes a `.scene` file
  pub fn save(&self) -> Result<String, FormatError> {
    file::save(&SceneFile::Dmd(self.clone()))
  }

  /// Checks what a definition's types can't say for themselves. Scenes from outside the program need this;
  /// [`DmdScene::load`] already does it.
  pub fn check(&self) -> Result<(), DmdSceneError> {
    self.composition.check().map_err(DmdSceneError::Composition)?;
    match self.composition.layers.iter().enumerate().find(|(_, layer)| layer.targets.is_some()) {
      Some((index, layer)) => Err(DmdSceneError::LayerTargets { layer: layer.label(index) }),
      None => Ok(()),
    }
  }
}

/// Why a scene can't be played
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DmdSceneError {
  Composition(SceneError),
  /// A layer chooses between targets, which are only for LEDs
  LayerTargets { layer: String },
}

impl fmt::Display for DmdSceneError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Composition(err) => write!(f, "{err}"),
      Self::LayerTargets { layer } => write!(f, "{layer} chooses targets, which a DMD doesn't have"),
    }
  }
}

impl std::error::Error for DmdSceneError {}

/// How a file says it's a DMD scene, which is also what keeps LEDs from reading one
#[derive(Serialize, Deserialize)]
enum SceneFile {
  Dmd(DmdScene),
}

#[cfg(test)]
mod tests {
  use frontbox_gfx::prelude::*;
  use frontbox_gfx::scene::*;

  use super::*;

  fn fill() -> Fill {
    Fill::new(FillNode::Solid(Color(Rgba::red()).into()))
  }

  #[test]
  fn survives_a_save() {
    let written = DmdScene::new(GfxComposition::new(vec![Layer::new(fill()).name("sky")])).save().unwrap();
    assert!(written.starts_with("Dmd("), "{written}");
    let read = DmdScene::load(&written).unwrap();
    assert_eq!(read.composition.layers[0].name.as_deref(), Some("sky"));
  }

  #[test]
  fn an_led_scene_is_refused() {
    let result = DmdScene::load("Led(version: 1, viewport: Sequence, composition: (layers: []))");
    assert!(matches!(result, Err(FormatError::Read(_))), "{result:?}");
  }

  #[test]
  fn a_layer_cant_choose_targets() {
    let scene = DmdScene::new(GfxComposition::new(vec![Layer::new(fill()).targets(["ring"])]));
    assert_eq!(scene.check(), Err(DmdSceneError::LayerTargets { layer: "layer 1".into() }));
  }
}
