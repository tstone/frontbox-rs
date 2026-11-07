use std::fmt;

use frontbox_core::prelude::*;
use frontbox_gfx::scene::file::{self, FormatError};
use frontbox_gfx::scene::{GfxComposition, SceneError, Viewport};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// What LEDs show: a composition and the LEDs it's on, as written to a `.scene` file. [`LedPlayback`](crate::LedPlayback)
/// plays one.
///
/// ```rust
/// # use frontbox_led::prelude::*;
/// let scene = LedScene::load(r#"Led(
///   version: 1,
///   viewport: Sequence,
///   targets: Single(leds: Tag("Playfield"), z: 1),
///   composition: (layers: [(fill: (base: Solid(Fixed(255, 0, 0, 255))))]),
/// )"#).unwrap();
///
/// let program = LedPlayback::new(scene).unwrap();
/// ```
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedScene {
  pub version: u32,
  /// How the LEDs land on the composition: along a `Sequence` in order, or on an `Area` by where they physically are
  pub viewport: Viewport,
  #[serde(default)]
  pub targets: LedTargets,
  pub composition: GfxComposition,
}

impl LedScene {
  /// A scene of the current format version
  pub fn new(viewport: Viewport, targets: LedTargets, composition: GfxComposition) -> Self {
    Self { version: file::VERSION, viewport, targets, composition }
  }

  /// Reads a `.scene` file, which has to be an LED scene that can be played
  pub fn load(source: &str) -> Result<Self, FormatError> {
    let SceneFile::Led(scene) = file::load(source)?;
    file::check_version(scene.version)?;
    scene.check().map_err(|err| FormatError::Invalid(err.to_string()))?;
    Ok(scene)
  }

  /// Writes a `.scene` file
  pub fn save(&self) -> Result<String, FormatError> {
    // only `load` can take it back, so it isn't cloned for anything else
    file::save(&SceneFile::Led(self.clone()))
  }

  /// Checks what a definition's types can't say for themselves. Scenes from outside the program need this;
  /// [`LedScene::load`] already does it.
  pub fn check(&self) -> Result<(), LedSceneError> {
    self.composition.check().map_err(LedSceneError::Composition)?;
    for (name, target) in self.targets.iter() {
      match (self.viewport, &target.plane) {
        (Viewport::Area, None) => return Err(LedSceneError::NoPlane { target: name.map(str::to_string) }),
        (Viewport::Sequence, Some(_)) => return Err(LedSceneError::UnusedPlane { target: name.map(str::to_string) }),
        _ => {}
      }
    }
    for (index, layer) in self.composition.layers.iter().enumerate() {
      for target in layer.targets.iter().flatten() {
        if !self.targets.iter().any(|(name, _)| name == Some(target)) {
          return Err(LedSceneError::UnknownTarget { layer: layer.label(index), target: target.clone() });
        }
      }
    }
    Ok(())
  }
}

/// What a scene plays on, which says how much of that code can change
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum LedTargets {
  /// Nothing yet. It can't be played, and the editor previews it on every LED.
  #[default]
  None,
  /// One set of LEDs, which code can replace with [`LedPlayback::with_leds`](crate::LedPlayback::with_leds)
  Single(LedTarget),
  /// Several sets of LEDs at once on the one timeline, by name, which layers choose between. These stay as they're
  /// written: something code has to change needs to be a scene with a single target. Later ones are drawn on top.
  Multiple(#[cfg_attr(feature = "ts", ts(as = "std::collections::HashMap<String, LedTarget>"))] IndexMap<String, LedTarget>),
}

impl LedTargets {
  /// Each target with its name, if it has one
  pub fn iter(&self) -> impl Iterator<Item = (Option<&str>, &LedTarget)> {
    let (single, multiple) = match self {
      Self::None => (None, None),
      Self::Single(target) => (Some(target), None),
      Self::Multiple(targets) => (None, Some(targets)),
    };
    let named = multiple.into_iter().flatten().map(|(name, target)| (Some(name.as_str()), target));
    single.map(|target| (None, target)).into_iter().chain(named)
  }
}

/// Some LEDs a scene plays on
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedTarget {
  pub leds: LedQ,
  /// The layer to declare them on
  #[serde(default, skip_serializing_if = "is_zero")]
  #[cfg_attr(feature = "ts", ts(as = "Option<i8>", optional))]
  pub z: i8,
  /// The plane an `Area` scene covers, by the name it was added to the app under, which gives the scene its size and the
  /// LEDs their positions. A `Sequence` has no use for one.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub plane: Option<String>,
}

impl LedTarget {
  pub fn new(leds: LedQ) -> Self {
    Self { leds, z: 0, plane: None }
  }

  pub fn at_z(mut self, z: i8) -> Self {
    self.z = z;
    self
  }

  pub fn on_plane(mut self, plane: impl Into<String>) -> Self {
    self.plane = Some(plane.into());
    self
  }
}

fn is_zero(z: &i8) -> bool {
  *z == 0
}

/// Why a scene can't be played
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedSceneError {
  Composition(SceneError),
  /// A target of an `Area` scene has no plane to be placed on. `None` is a scene's single target.
  NoPlane { target: Option<String> },
  /// A target of a `Sequence` scene says a plane it has no use for
  UnusedPlane { target: Option<String> },
  /// A layer plays on a target the scene doesn't have
  UnknownTarget { layer: String, target: String },
}

impl fmt::Display for LedSceneError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let called = |target: &Option<String>| target.as_ref().map_or("the target".to_string(), |name| format!("target \"{name}\""));
    match self {
      Self::Composition(err) => write!(f, "{err}"),
      Self::NoPlane { target } => write!(f, "{} needs a plane, as the scene covers an area", called(target)),
      Self::UnusedPlane { target } => write!(f, "{} has a plane, which a sequence doesn't use", called(target)),
      Self::UnknownTarget { layer, target } => write!(f, "{layer} plays on \"{target}\", which isn't one of the scene's targets"),
    }
  }
}

impl std::error::Error for LedSceneError {}

/// How a file says it's an LED scene, which is also what keeps a DMD from reading one
#[derive(Serialize, Deserialize)]
enum SceneFile {
  Led(LedScene),
}

#[cfg(test)]
mod tests {
  use frontbox_gfx::prelude::*;
  use frontbox_gfx::scene::*;

  use super::*;

  fn fill() -> Fill {
    Fill::new(FillNode::Solid(Color(Rgba::red()).into()))
  }

  fn two_targets() -> LedTargets {
    LedTargets::Multiple(IndexMap::from([
      ("ring".to_string(), LedTarget::new(LedQ::tag::<frontbox_core::tags::Playfield>())),
      ("center".to_string(), LedTarget::new(LedQ::names(["a", "b"])).at_z(2)),
    ]))
  }

  fn sequence(targets: LedTargets, layers: Vec<Layer>) -> LedScene {
    LedScene::new(Viewport::Sequence, targets, GfxComposition::new(layers))
  }

  #[test]
  fn a_file_says_what_kind_of_scene_it_is() {
    let written = sequence(LedTargets::None, vec![]).save().unwrap();
    assert!(written.starts_with("Led("), "{written}");
  }

  #[test]
  fn a_file_of_another_kind_is_refused() {
    let result = LedScene::load("Dmd(version: 1, composition: (layers: []))");
    assert!(matches!(result, Err(FormatError::Read(_))), "{result:?}");
  }

  #[test]
  fn a_newer_file_is_refused() {
    let result = LedScene::load("Led(version: 99, viewport: Sequence, composition: (layers: []))");
    assert!(matches!(result, Err(FormatError::Newer { version: 99 })), "{result:?}");
  }

  #[test]
  fn a_scene_with_no_targets_says_so() {
    let scene = LedScene::load("Led(version: 1, viewport: Sequence, composition: (layers: []))").unwrap();
    assert!(matches!(scene.targets, LedTargets::None));
  }

  #[test]
  fn a_single_target_survives_a_save() {
    let target = LedTarget::new(LedQ::names(["b", "a"]).reverse()).at_z(-1);
    let read = LedScene::load(&sequence(LedTargets::Single(target.clone()), vec![]).save().unwrap()).unwrap();
    assert!(matches!(read.targets, LedTargets::Single(found) if found == target));
  }

  #[test]
  fn multiple_targets_survive_a_save_in_order() {
    let layers = vec![Layer::new(fill()), Layer::new(fill()).targets(["center"])];
    let read = LedScene::load(&sequence(two_targets(), layers).save().unwrap()).unwrap();

    let names: Vec<_> = read.targets.iter().map(|(name, _)| name.unwrap().to_string()).collect();
    assert_eq!(names, ["ring", "center"]);
    assert_eq!(read.composition.layers[0].targets, None);
    assert_eq!(read.composition.layers[1].targets, Some(vec!["center".to_string()]));
    assert_eq!(read.targets.iter().nth(1).unwrap().1.z, 2);
  }

  #[test]
  fn a_tag_is_written_by_its_name() {
    let written = sequence(two_targets(), vec![]).save().unwrap();
    assert!(written.contains(r#"Tag("Playfield")"#), "{written}");
  }

  #[test]
  fn a_plane_is_written_by_its_name() {
    let target = LedTarget::new(LedQ::Every).on_plane("Playfield");
    let scene = LedScene::new(Viewport::Area, LedTargets::Single(target), GfxComposition::new(vec![]));
    assert!(scene.save().unwrap().contains(r#"plane: "Playfield""#));
  }

  #[test]
  fn an_area_needs_a_plane_to_be_on() {
    let target = LedTarget::new(LedQ::Every);
    let scene = LedScene::new(Viewport::Area, LedTargets::Single(target), GfxComposition::new(vec![]));
    assert_eq!(scene.check(), Err(LedSceneError::NoPlane { target: None }));
  }

  #[test]
  fn a_sequence_has_no_use_for_a_plane() {
    let scene = sequence(LedTargets::Single(LedTarget::new(LedQ::Every).on_plane("Playfield")), vec![]);
    assert_eq!(scene.check(), Err(LedSceneError::UnusedPlane { target: None }));
  }

  #[test]
  fn a_layer_must_name_one_of_the_scenes_targets() {
    let scene = sequence(two_targets(), vec![Layer::new(fill()).targets(["middle"])]);
    assert_eq!(scene.check(), Err(LedSceneError::UnknownTarget { layer: "layer 1".into(), target: "middle".into() }));
  }

  #[test]
  fn only_multiple_targets_have_names_to_choose_between() {
    let scene = sequence(LedTargets::Single(LedTarget::new(LedQ::Every)), vec![Layer::new(fill()).targets(["ring"])]);
    assert!(matches!(scene.check(), Err(LedSceneError::UnknownTarget { .. })));
  }

  #[test]
  fn what_the_composition_cant_play_is_refused_too() {
    let composition = GfxComposition::new(vec![]).timing(Timing::new(0.0, frontbox_gfx::animation::Cycle::Once));
    let scene = LedScene::new(Viewport::Sequence, LedTargets::None, composition);
    assert_eq!(scene.check(), Err(LedSceneError::Composition(SceneError::NoDuration)));
  }
}
