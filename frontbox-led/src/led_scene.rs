use frontbox_core::prelude::*;
use frontbox_gfx::scene::file::{self, FormatError};
use frontbox_gfx::scene::{self, GfxComposition, Layer, Params, SceneError, Timing};
use serde::{Deserialize, Serialize};

/// What LEDs show, as written to a `.scene` file: layers that each say which LEDs they're on, and how, joined together on
/// one timeline. [`LedPlayback`](crate::LedPlayback) plays one.
///
/// ```rust
/// # use frontbox_led::prelude::*;
/// let scene = LedScene::load(r#"Led(
///   version: 1,
///   layers: [(
///     leds: Tag("Playfield"),
///     z: 1,
///     surface: Sequence,
///     layer: (fill: (base: Solid(Fixed(255, 0, 0, 255)))),
///   )],
/// )"#).unwrap();
///
/// let program = LedPlayback::new(scene).unwrap();
/// ```
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedScene {
  pub version: u32,
  /// Parameter names and their defaults
  #[serde(default, skip_serializing_if = "Params::is_empty")]
  #[cfg_attr(feature = "ts", ts(as = "Option<Params>", optional))]
  pub params: Params,
  /// `None` is static, and its tracks stay on their first moment
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub timing: Option<Timing>,
  /// Where they share LEDs and a z, they're drawn bottom to top, alpha over
  #[serde(default)]
  pub layers: Vec<LedLayer>,
}

impl LedScene {
  /// A scene of the current format version
  pub fn new(layers: Vec<LedLayer>) -> Self {
    Self { version: file::VERSION, params: Params::new(), timing: None, layers }
  }

  pub fn timing(mut self, timing: Timing) -> Self {
    self.timing = Some(timing);
    self
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
  pub fn check(&self) -> Result<(), SceneError> {
    self.composition().check()
  }

  /// What's drawn, leaving out where. [`GfxComposition::only`] has one layer of it, to sample on its own surface.
  pub fn composition(&self) -> GfxComposition {
    let layers = self.layers.iter().map(|layer| layer.layer.clone()).collect();
    GfxComposition { params: self.params.clone(), timing: self.timing, layers }
  }
}

/// A layer and where it happens: a scene of its own, but for the timeline
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedLayer {
  /// `None` is none yet. The scene can't be played, and the editor previews the layer on every LED.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub leds: Option<LedQ>,
  /// Which layer of the LED system to declare them on
  #[serde(default, skip_serializing_if = "is_zero")]
  #[cfg_attr(feature = "ts", ts(as = "Option<i8>", optional))]
  pub z: i8,
  pub surface: Surface,
  pub layer: Layer,
}

impl LedLayer {
  /// Spreads along `leds` in order
  pub fn sequence(leds: LedQ, layer: Layer) -> Self {
    Self { leds: Some(leds), z: 0, surface: Surface::Sequence, layer }
  }

  /// Covers a plane, by the name it was added to the app under, lighting `leds` from where they are on it
  pub fn area(leds: LedQ, plane: impl Into<String>, layer: Layer) -> Self {
    Self { leds: Some(leds), z: 0, surface: Surface::Area(plane.into()), layer }
  }

  pub fn at_z(mut self, z: i8) -> Self {
    self.z = z;
    self
  }
}

/// What a layer is drawn on, which is what its numbers are measured against
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Surface {
  /// Its LEDs in order
  Sequence,
  /// A plane, by the name it was added to the app under, which gives the layer its size and the LEDs their positions
  Area(String),
}

impl Surface {
  pub fn scene_surface(&self) -> scene::Surface {
    match self {
      Self::Sequence => scene::Surface::Sequence,
      Self::Area(_) => scene::Surface::Area,
    }
  }

  pub fn plane(&self) -> Option<&str> {
    match self {
      Self::Sequence => None,
      Self::Area(plane) => Some(plane),
    }
  }
}

fn is_zero(z: &i8) -> bool {
  *z == 0
}

/// How a file says it's an LED scene, which is also what keeps a DMD from reading one
#[derive(Serialize, Deserialize)]
enum SceneFile {
  Led(LedScene),
}

#[cfg(test)]
mod tests {
  use frontbox_gfx::animation::Cycle;
  use frontbox_gfx::prelude::*;
  use frontbox_gfx::scene::*;

  use super::*;
  use super::Surface;

  fn layer() -> Layer {
    Layer::new(Fill::new(FillNode::Solid(Color(Rgba::red()).into())))
  }

  #[test]
  fn a_file_says_what_kind_of_scene_it_is() {
    let written = LedScene::new(vec![]).save().unwrap();
    assert!(written.starts_with("Led("), "{written}");
  }

  #[test]
  fn a_file_of_another_kind_is_refused() {
    let result = LedScene::load("Dmd(version: 1, composition: (layers: []))");
    assert!(matches!(result, Err(FormatError::Read(_))), "{result:?}");
  }

  #[test]
  fn a_newer_file_is_refused() {
    let result = LedScene::load("Led(version: 99, layers: [])");
    assert!(matches!(result, Err(FormatError::Newer { version: 99 })), "{result:?}");
  }

  #[test]
  fn a_layer_with_no_leds_says_so() {
    let source = "Led(version: 1, layers: [(surface: Sequence, layer: (fill: (base: Solid(Fixed(255, 0, 0, 255)))))])";
    assert_eq!(LedScene::load(source).unwrap().layers[0].leds, None);
  }

  #[test]
  fn a_layer_survives_a_save() {
    let scene = LedScene::new(vec![LedLayer::sequence(LedQ::names(["b", "a"]).reverse(), layer()).at_z(-1)]);
    let read = LedScene::load(&scene.save().unwrap()).unwrap();
    assert_eq!(read.layers[0].leds, scene.layers[0].leds);
    assert_eq!(read.layers[0].z, -1);
    assert_eq!(read.layers[0].surface, Surface::Sequence);
  }

  #[test]
  fn layers_survive_a_save_in_order() {
    let scene = LedScene::new(vec![
      LedLayer::sequence(LedQ::name("a"), layer().name("under")),
      LedLayer::area(LedQ::Every, "Backbox", layer().name("over")),
    ]);
    let read = LedScene::load(&scene.save().unwrap()).unwrap();
    let names: Vec<_> = read.layers.iter().map(|layer| layer.layer.name.clone().unwrap()).collect();
    assert_eq!(names, ["under", "over"]);
    assert_eq!(read.layers[1].surface, Surface::Area("Backbox".into()));
  }

  #[test]
  fn a_tag_is_written_by_its_name() {
    let scene = LedScene::new(vec![LedLayer::sequence(LedQ::tag::<frontbox_core::tags::Playfield>(), layer())]);
    let written = scene.save().unwrap();
    assert!(written.contains(r#"Tag("Playfield")"#), "{written}");
  }

  #[test]
  fn a_plane_is_written_by_its_name() {
    let scene = LedScene::new(vec![LedLayer::area(LedQ::Every, "Playfield", layer())]);
    let written = scene.save().unwrap();
    assert!(written.contains(r#"surface: Area("Playfield")"#), "{written}");
  }

  #[test]
  fn a_surface_says_how_a_layer_is_seen() {
    assert_eq!(Surface::Sequence.scene_surface(), scene::Surface::Sequence);
    assert_eq!(Surface::Sequence.plane(), None);
    assert_eq!(Surface::Area("Backbox".into()).scene_surface(), scene::Surface::Area);
    assert_eq!(Surface::Area("Backbox".into()).plane(), Some("Backbox"));
  }

  #[test]
  fn what_a_layer_cant_play_is_refused_by_its_place_in_the_scene() {
    let track = Prop::Track(Track { keys: vec![], cycle: Cycle::Once });
    let changing = Fill::new(FillNode::Solid(Color(Rgba::red()).into())).adjust(ColorAdjust::HueShift(track));
    let scene = LedScene::new(vec![LedLayer::sequence(LedQ::Every, layer()), LedLayer::sequence(LedQ::Every, Layer::new(changing))]);
    assert_eq!(scene.check(), Err(SceneError::NoTiming { layer: "layer 2".into() }));
  }

  #[test]
  fn a_timing_with_no_length_is_refused() {
    let scene = LedScene::new(vec![]).timing(Timing::new(0.0, Cycle::Once));
    assert_eq!(scene.check(), Err(SceneError::NoDuration));
  }
}
