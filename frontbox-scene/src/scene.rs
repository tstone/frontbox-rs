use std::fmt;

use frontbox_core::prelude::*;
use frontbox_gfx::animation::Cycle;
use frontbox_gfx::scene::file::{self, FormatError};
use frontbox_gfx::scene::{
  self, Assets, Content, EnvelopeSource, Inputs, Layer, LayerAssetError, LayerFrame, Modulators,
  Placement, PreparedEnvelope, SceneError, SceneFileError, Timing, Value, Values, Variables,
  Visits,
};
use serde::{Deserialize, Serialize};

use crate::{Moment, SoundLayer, SoundStart};

/// Layers on one timeline, each saying what it's drawn on or that it's sound, as written to a `.scene` file. Each system
/// plays the layers it knows: `LightManager` plays [`LightLayer`]s, a display plays the [`DisplayLayer`]s for it, and
/// `SoundSystem` plays [`SoundLayer`]s. A [`Playback`](crate::Playback) holds what's only true while one plays.
///
/// ```rust
/// # use frontbox_scene::*;
/// let scene = Scene::load(r#"Scene(
///   version: 2,
///   timing: (duration: 0.5),
///   layers: [
///     Lights(lights: Tag("Playfield"), z: 1, surface: Sequence, layer: (fill: (base: Solid(Fixed(255, 0, 0, 255))))),
///     Sound(sound: "sounds/zap.mp3"),
///   ],
/// )"#).unwrap();
/// ```
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
  pub version: u32,
  /// Variable names and their defaults. A default can point at one of `values`. Files from before variables were named
  /// so say `params`.
  #[serde(default, alias = "params", skip_serializing_if = "Variables::is_empty")]
  #[cfg_attr(feature = "ts", ts(as = "Option<Variables>", optional))]
  pub variables: Variables,
  /// Named values, each a type and a prop of it, which a variable can point at, from its default or the code
  #[serde(default, skip_serializing_if = "Values::is_empty")]
  #[cfg_attr(feature = "ts", ts(as = "Option<Values>", optional))]
  pub values: Values,
  /// Waves by name, each a shape and a rate, which any number of modulated props can move by
  #[serde(default, skip_serializing_if = "Modulators::is_empty")]
  #[cfg_attr(feature = "ts", ts(as = "Option<Modulators>", optional))]
  pub modulators: Modulators,
  /// `None` is static, and its keyframes stay on their first key
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub timing: Option<Timing>,
  /// Drawn bottom to top where they land on the same thing at the same z
  #[serde(default)]
  pub layers: Vec<SceneLayer>,
  /// The decoded images and loaded fonts its layers draw, from [`Scene::with_assets`]. They aren't written to the file,
  /// which only names them.
  #[serde(skip)]
  #[cfg_attr(feature = "ts", ts(skip))]
  pub assets: Assets,
}

impl Scene {
  /// A scene of the current format version
  pub fn new(layers: Vec<SceneLayer>) -> Self {
    Self {
      version: file::VERSION,
      variables: Variables::new(),
      values: Values::new(),
      modulators: Modulators::new(),
      timing: None,
      layers,
      assets: Assets::new(),
    }
  }

  pub fn timing(mut self, timing: Timing) -> Self {
    self.timing = Some(timing);
    self
  }

  /// Reads a `.scene` file, which has to be one that can be played
  pub fn load(source: &str) -> Result<Self, FormatError> {
    let SceneFile::Scene(scene) = file::load(source)?;
    file::check_version(scene.version)?;
    scene
      .check()
      .map_err(|err| FormatError::Invalid(err.to_string()))?;
    Ok(scene)
  }

  /// Reads scene `name` from `content`, with the images and fonts it draws. That reads files, so do it ahead of time
  /// rather than in a tick.
  pub fn open(content: &Content, name: &str) -> Result<Self, OpenError> {
    let source = content.read_scene(name).map_err(OpenError::File)?;
    let scene = Self::load(&source).map_err(|error| OpenError::Format {
      name: name.into(),
      error,
    })?;
    scene
      .with_assets(content)
      .map_err(|error| OpenError::Assets {
        name: name.into(),
        error,
      })
  }

  /// Writes a `.scene` file
  pub fn save(&self) -> Result<String, FormatError> {
    // only `load` can take it back, so it isn't cloned for anything else
    file::save(&SceneFile::Scene(self.clone()))
  }

  /// Checks what a definition's types can't say for themselves. Scenes from outside the program need this;
  /// [`Scene::load`] already does it.
  pub fn check(&self) -> Result<(), SceneError> {
    if let Some(timing) = self.timing {
      timing.check()?;
    }
    for (name, modulator) in &self.modulators {
      modulator.check(name)?;
    }
    if let Some(name) = self
      .values
      .iter()
      .find(|(_, value)| self.timing.is_none() && value.is_animated())
      .map(|(name, _)| name)
    {
      return Err(SceneError::NoTiming {
        layer: format!("value \"{name}\""),
      });
    }
    self
      .layers
      .iter()
      .enumerate()
      .try_for_each(|(index, layer)| match layer {
        SceneLayer::Sound(sound) => sound.check(index, self.timing),
        drawn => drawn
          .layer()
          .map_or(Ok(()), |layer| layer.check(index, self.timing)),
      })?;
    self.inputs().envelopes.iter().try_for_each(|key| {
      key.check()?;
      let EnvelopeSource::Sound { layer, .. } = &key.source;
      match self.sound_layer(layer) {
        Some(_) => Ok(()),
        None => Err(SceneError::NoSoundToFollow {
          sound: layer.clone(),
        }),
      }
    })
  }

  /// The first sound layer named `name`, and where it is among the layers
  fn sound_layer(&self, name: &str) -> Option<(usize, &SoundLayer)> {
    self
      .layers
      .iter()
      .enumerate()
      .filter_map(|(index, layer)| Some((index, layer.as_sound()?)))
      .find(|(_, layer)| layer.name.as_deref() == Some(name))
  }

  /// What its layers and named values read from outside them: variables from the code playing it, and the external
  /// envelopes they follow
  pub fn inputs(&self) -> Inputs {
    let mut inputs = Inputs::new();
    self
      .values
      .values()
      .for_each(|value| value.read_inputs(&mut inputs));
    for layer in &self.layers {
      match layer {
        SceneLayer::Sound(_) => {}
        drawn => drawn
          .layer()
          .iter()
          .for_each(|layer| layer.read_inputs(&mut inputs)),
      }
    }
    inputs
  }

  /// Logs a warning if `value` points `variable` at a named value the scene doesn't have. The variable then reads as its
  /// type's zero value.
  pub(crate) fn warn_if_missing(&self, variable: &str, value: &Value) {
    if let Value::Named(name) = value
      && !self.values.contains_key(name)
    {
      let names: Vec<_> = self.values.keys().collect();
      log::warn!(target: "frontbox::scene", "Variable \"{variable}\" points at value \"{name}\", which the scene doesn't have. Its values: {names:?}");
    }
  }

  /// Names of the images it uses, possibly with repeats
  pub fn image_names(&self) -> impl Iterator<Item = &str> {
    self
      .layers
      .iter()
      .filter_map(SceneLayer::layer)
      .flat_map(Layer::image_names)
  }

  /// Names of the fonts its text is in, possibly with repeats
  pub fn font_names(&self) -> impl Iterator<Item = &str> {
    self
      .layers
      .iter()
      .filter_map(SceneLayer::layer)
      .flat_map(Layer::font_names)
  }

  /// Names of the sounds its layers play, possibly with repeats
  pub fn sound_names(&self) -> impl Iterator<Item = &str> {
    self
      .layers
      .iter()
      .filter_map(SceneLayer::as_sound)
      .map(|layer| layer.sound.as_str())
  }

  /// The sounds that start after `after` and up to `moment`'s time, in the order they start. Pass the time of the last
  /// call as `after`, so each start is given once, or `None` for a playback that has just started.
  pub fn sounds_started<'a>(
    &'a self,
    moment: &Moment<'a>,
    after: Option<f32>,
  ) -> Vec<SoundStart<'a>> {
    let layers = self.layers.iter().filter_map(SceneLayer::as_sound);
    let starts = layers.filter_map(|layer| {
      layer
        .started(self.timing, after, moment.time)
        .map(|start| (layer, start))
    });
    sound_starts(starts)
  }

  /// Where each sound starts in cycle `cycle` (counting from zero), in the order they start, for previewing a cycle. A
  /// cycle past the last is the last.
  pub fn sounds_in_cycle(&self, cycle: u32) -> Vec<SoundStart<'_>> {
    let top = self.timing.map_or(0.0, |timing| {
      let cycle = match timing.cycle {
        Cycle::Once => 0,
        Cycle::Times(times) => cycle.min(times.saturating_sub(1)),
        Cycle::Forever => cycle,
      };
      cycle as f32 * timing.duration
    });
    let layers = self.layers.iter().filter_map(SceneLayer::as_sound);
    let starts = layers.map(|layer| (layer, top + layer.start));
    sound_starts(starts)
  }

  /// Loads the images and fonts its layers draw from `content`, and prepares the external envelopes they follow from
  /// the sounds of its sound layers. A layer whose image, font, or sound is missing is an error, rather than one that
  /// silently draws nothing. That reads and decodes files, so do it ahead of time rather than in a tick.
  ///
  /// An envelope following a sound layer the scene doesn't have is left out, to rest at its spread's default.
  /// [`Scene::check`] reports it.
  pub fn with_assets(mut self, content: &Content) -> Result<Self, LayerAssetError> {
    let mut assets = content.assets_for(self.layers.iter().filter_map(SceneLayer::layer))?;
    for key in self.inputs().envelopes {
      let EnvelopeSource::Sound { layer, analysis } = &key.source;
      let Some((index, sound)) = self.sound_layer(layer) else {
        continue;
      };
      let analyzed =
        content
          .envelope(&sound.sound, *analysis)
          .map_err(|error| LayerAssetError {
            layer: sound.label(index),
            error,
          })?;
      let placement = Placement {
        start: sound.start,
        skip: sound.skip,
        end: sound.end,
      };
      let prepared = PreparedEnvelope::new(&analyzed, placement, key.attack(), key.release());
      assets.insert_envelope(key, prepared);
    }
    self.assets = assets;
    Ok(self)
  }

  /// Whether every cycle has played by `time`. A scene that's static or repeats forever never completes.
  pub fn is_complete(&self, time: f32) -> bool {
    self.timing.is_some_and(|timing| timing.is_complete(time))
  }

  /// Moves every light layer to `lights`, for when which lights a scene plays on is only known in the game. Each layer's z and
  /// surface stay as the scene says. That's only for a scene whose light layers are all on the same lights, as there's
  /// otherwise no saying which of them to move.
  pub fn moved_to_lights(mut self, lights: LightQ) -> Result<Self, DifferentLights> {
    let mut on = self
      .layers
      .iter()
      .filter_map(SceneLayer::as_lights)
      .map(|layer| &layer.lights);
    if let Some(first) = on.next()
      && on.any(|other| other != first)
    {
      return Err(DifferentLights);
    }
    for layer in self.layers.iter_mut().filter_map(SceneLayer::as_lights_mut) {
      layer.lights = Some(lights.clone());
    }
    Ok(self)
  }

  /// What to sample one of its layers with at `moment`, drawn on `surface` of `size`. Pass the layer's
  /// [`Layer::visits`] in `visits` for a layer with a graph.
  pub fn frame<'a>(
    &'a self,
    moment: &Moment<'a>,
    surface: scene::Surface,
    size: Vec2,
    visits: Option<&'a Visits>,
  ) -> LayerFrame<'a> {
    LayerFrame {
      time: moment.time,
      timing: self.timing,
      playback_seed: moment.playback_seed,
      variables: moment.variables,
      defaults: &self.variables,
      values: &self.values,
      modulators: &self.modulators,
      assets: &self.assets,
      size,
      surface,
      visits,
    }
  }
}

/// A layer, and what it's drawn on or heard through
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SceneLayer {
  Lights(LightLayer),
  Display(DisplayLayer),
  Sound(SoundLayer),
}

impl SceneLayer {
  /// What's drawn, for a light or display layer
  pub fn layer(&self) -> Option<&Layer> {
    match self {
      Self::Lights(layer) => Some(&layer.layer),
      Self::Display(layer) => Some(&layer.layer),
      Self::Sound(_) => None,
    }
  }

  pub fn layer_mut(&mut self) -> Option<&mut Layer> {
    match self {
      Self::Lights(layer) => Some(&mut layer.layer),
      Self::Display(layer) => Some(&mut layer.layer),
      Self::Sound(_) => None,
    }
  }

  /// Order among everything playing on the same thing: higher is on top. `None` for a sound layer.
  pub fn z(&self) -> Option<i8> {
    match self {
      Self::Lights(layer) => Some(layer.z),
      Self::Display(layer) => Some(layer.z),
      Self::Sound(_) => None,
    }
  }

  /// How logs and errors refer to the layer at `index`
  pub fn label(&self, index: usize) -> String {
    match self {
      Self::Sound(layer) => layer.label(index),
      drawn => drawn
        .layer()
        .map(|layer| layer.label(index))
        .unwrap_or_default(),
    }
  }

  pub fn as_lights(&self) -> Option<&LightLayer> {
    match self {
      Self::Lights(layer) => Some(layer),
      _ => None,
    }
  }

  pub fn as_lights_mut(&mut self) -> Option<&mut LightLayer> {
    match self {
      Self::Lights(layer) => Some(layer),
      _ => None,
    }
  }

  pub fn as_display(&self) -> Option<&DisplayLayer> {
    match self {
      Self::Display(layer) => Some(layer),
      _ => None,
    }
  }

  pub fn as_display_mut(&mut self) -> Option<&mut DisplayLayer> {
    match self {
      Self::Display(layer) => Some(layer),
      _ => None,
    }
  }

  pub fn as_sound(&self) -> Option<&SoundLayer> {
    match self {
      Self::Sound(layer) => Some(layer),
      _ => None,
    }
  }
}

impl From<LightLayer> for SceneLayer {
  fn from(layer: LightLayer) -> Self {
    Self::Lights(layer)
  }
}

impl From<DisplayLayer> for SceneLayer {
  fn from(layer: DisplayLayer) -> Self {
    Self::Display(layer)
  }
}

impl From<SoundLayer> for SceneLayer {
  fn from(layer: SoundLayer) -> Self {
    Self::Sound(layer)
  }
}

/// A layer drawn on lights
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightLayer {
  /// `None` is none yet. It lands on no lights, and the editor previews it on every light.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub lights: Option<LightQ>,
  /// Which z-layer of light management it's on
  #[serde(default, skip_serializing_if = "is_zero")]
  #[cfg_attr(feature = "ts", ts(as = "Option<i8>", optional))]
  pub z: i8,
  pub surface: LightSurface,
  pub layer: Layer,
}

impl LightLayer {
  /// Spreads along `lights` in order
  pub fn sequence(lights: LightQ, layer: Layer) -> Self {
    Self {
      lights: Some(lights),
      z: 0,
      surface: LightSurface::Sequence,
      layer,
    }
  }

  /// Covers a plane, by the name it was added to the app under, lighting `lights` from where they are on it
  pub fn area(lights: LightQ, plane: impl Into<String>, layer: Layer) -> Self {
    Self {
      lights: Some(lights),
      z: 0,
      surface: LightSurface::Area(plane.into()),
      layer,
    }
  }

  pub fn at_z(mut self, z: i8) -> Self {
    self.z = z;
    self
  }
}

/// What a light layer is drawn on, which is what its numbers are measured against
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightSurface {
  /// Its lights in order
  Sequence,
  /// A plane, by the name it was added to the app under, which gives the layer its size and the lights their positions
  Area(String),
}

impl LightSurface {
  /// The kind of surface the layer is sampled on, which the plane's name doesn't change
  pub fn sampled_as(&self) -> scene::Surface {
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

/// A layer drawn on a display, such as a DMD. Its size is the display's. Nothing plays these yet.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayLayer {
  /// The display, by the name it was added to the app under
  pub display: String,
  /// Order among everything playing on the display: higher is on top
  #[serde(default, skip_serializing_if = "is_zero")]
  #[cfg_attr(feature = "ts", ts(as = "Option<i8>", optional))]
  pub z: i8,
  pub layer: Layer,
}

impl DisplayLayer {
  pub fn new(display: impl Into<String>, layer: Layer) -> Self {
    Self {
      display: display.into(),
      z: 0,
      layer,
    }
  }

  pub fn at_z(mut self, z: i8) -> Self {
    self.z = z;
    self
  }
}

/// Why [`Scene::moved_to_lights`] refused: the scene's light layers aren't all on the same lights
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifferentLights;

impl fmt::Display for DifferentLights {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "a scene with layers on different lights can't be moved to other lights"
    )
  }
}

impl std::error::Error for DifferentLights {}

/// Each layer's sound starting at its time, sorted by when they start
fn sound_starts<'a>(starts: impl Iterator<Item = (&'a SoundLayer, f32)>) -> Vec<SoundStart<'a>> {
  let mut started: Vec<_> = starts
    .map(|(layer, time)| SoundStart {
      sound: &layer.sound,
      time,
      volume: layer.volume.max(0.0),
      skip: layer.skip,
      length: layer.length(),
    })
    .collect();
  started.sort_by(|a, b| a.time.total_cmp(&b.time));
  started
}

fn is_zero(z: &i8) -> bool {
  *z == 0
}

/// Why [`Scene::open`] couldn't open a scene
#[derive(Debug)]
pub enum OpenError {
  File(SceneFileError),
  Format {
    name: String,
    error: FormatError,
  },
  Assets {
    name: String,
    error: LayerAssetError,
  },
}

impl fmt::Display for OpenError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::File(error) => error.fmt(f),
      Self::Format { name, error } => write!(f, "scene \"{name}\": {error}"),
      Self::Assets { name, error } => write!(f, "scene \"{name}\": {error}"),
    }
  }
}

impl std::error::Error for OpenError {}

/// How a file says it's a scene
#[derive(Serialize, Deserialize)]
enum SceneFile {
  Scene(Scene),
}

#[cfg(test)]
mod tests {
  use frontbox_gfx::prelude::*;
  use frontbox_gfx::scene::*;

  use super::*;

  fn layer() -> Layer {
    Layer::new(Fill::new(FillNode::Solid(Color(Rgba::red()).into())))
  }

  #[test]
  fn a_file_says_it_is_a_scene() {
    let written = Scene::new(vec![]).save().unwrap();
    assert!(written.starts_with("Scene("), "{written}");
  }

  #[test]
  fn a_file_of_the_old_kind_is_refused() {
    let result = Scene::load("Led(version: 1, layers: [])");
    assert!(matches!(result, Err(FormatError::Read(_))), "{result:?}");
  }

  #[test]
  fn a_newer_file_is_refused() {
    let result = Scene::load("Scene(version: 99, layers: [])");
    assert!(
      matches!(result, Err(FormatError::Newer { version: 99 })),
      "{result:?}"
    );
  }

  #[test]
  fn a_layer_with_no_lights_says_so() {
    let source = "Scene(version: 2, layers: [Lights(surface: Sequence, layer: (fill: (base: Solid(Fixed(255, 0, 0, 255)))))])";
    assert_eq!(
      Scene::load(source).unwrap().layers[0]
        .as_lights()
        .unwrap()
        .lights,
      None
    );
  }

  #[test]
  fn layers_survive_a_save_in_order() {
    let scene = Scene::new(vec![
      LightLayer::sequence(LightQ::names(["b", "a"]).reverse(), layer().name("under"))
        .at_z(-1)
        .into(),
      LightLayer::area(LightQ::Every, "Backbox", layer().name("over")).into(),
      DisplayLayer::new("Dmd", layer().name("screen"))
        .at_z(2)
        .into(),
    ]);
    let read = Scene::load(&scene.save().unwrap()).unwrap();
    let names: Vec<_> = read
      .layers
      .iter()
      .map(|layer| layer.layer().unwrap().name.clone().unwrap())
      .collect();
    assert_eq!(names, ["under", "over", "screen"]);

    let under = read.layers[0].as_lights().unwrap();
    assert_eq!(
      (&under.lights, under.z, &under.surface),
      (
        &scene.layers[0].as_lights().unwrap().lights,
        -1,
        &LightSurface::Sequence
      )
    );
    assert_eq!(
      read.layers[1].as_lights().unwrap().surface,
      LightSurface::Area("Backbox".into())
    );
    let screen = read.layers[2].as_display().unwrap();
    assert_eq!((screen.display.as_str(), screen.z), ("Dmd", 2));
  }

  #[derive(Tag)]
  struct Playfield;

  #[test]
  fn a_layer_says_what_its_on() {
    let written = Scene::new(vec![
      LightLayer::area(LightQ::tag::<Playfield>(), "Playfield", layer()).into(),
      DisplayLayer::new("Dmd", layer()).into(),
    ])
    .save()
    .unwrap();
    assert!(written.contains(r#"Lights("#), "{written}");
    assert!(written.contains(r#"Tag("Playfield")"#), "{written}");
    assert!(
      written.contains(r#"surface: Area("Playfield")"#),
      "{written}"
    );
    assert!(written.contains(r#"Display("#), "{written}");
    assert!(written.contains(r#"display: "Dmd""#), "{written}");
  }

  #[test]
  fn a_surface_says_how_a_layer_is_seen() {
    assert_eq!(
      LightSurface::Sequence.sampled_as(),
      scene::Surface::Sequence
    );
    assert_eq!(LightSurface::Sequence.plane(), None);
    assert_eq!(
      LightSurface::Area("Backbox".into()).sampled_as(),
      scene::Surface::Area
    );
    assert_eq!(
      LightSurface::Area("Backbox".into()).plane(),
      Some("Backbox")
    );
  }

  #[test]
  fn what_a_layer_cant_play_is_refused_by_its_place_in_the_scene() {
    let track = Prop::Keyframes(Keyframes {
      keys: vec![],
      cycle: Cycle::Once,
    });
    let changing =
      Fill::new(FillNode::Solid(Color(Rgba::red()).into())).adjust(ColorAdjust::HueShift(track));
    let scene = Scene::new(vec![
      DisplayLayer::new("Dmd", layer()).into(),
      LightLayer::sequence(LightQ::Every, Layer::new(changing)).into(),
    ]);
    assert_eq!(
      scene.check(),
      Err(SceneError::NoTiming {
        layer: "layer 2".into()
      })
    );
  }

  #[test]
  fn a_timing_with_no_length_is_refused() {
    let scene = Scene::new(vec![]).timing(Timing::new(0.0, Cycle::Once));
    assert_eq!(scene.check(), Err(SceneError::NoDuration));
  }

  #[test]
  fn a_modulator_slower_than_once_a_cycle_is_refused() {
    let source = r#"Scene(version: 2, timing: (duration: 1.0), modulators: {"drift": (waveform: (attack: Linear), rate: 0.5)})"#;
    let Err(FormatError::Invalid(message)) = Scene::load(source) else {
      panic!("loaded a modulator with a rate of 0.5");
    };
    assert!(message.contains("drift"), "{message}");
  }

  #[test]
  fn light_layers_on_the_same_lights_can_be_moved_together() {
    let scene = Scene::new(vec![
      LightLayer::sequence(LightQ::name("lane"), layer()).into(),
      DisplayLayer::new("Dmd", layer()).into(),
      LightLayer::area(LightQ::name("lane"), "Playfield", layer())
        .at_z(1)
        .into(),
    ]);
    let moved = scene.moved_to_lights(LightQ::name("other")).unwrap();
    let lights: Vec<_> = moved
      .layers
      .iter()
      .filter_map(SceneLayer::as_lights)
      .map(|layer| (layer.lights.clone(), layer.z))
      .collect();
    assert_eq!(
      lights,
      [
        (Some(LightQ::name("other")), 0),
        (Some(LightQ::name("other")), 1)
      ]
    );
  }

  #[test]
  fn light_layers_on_different_lights_cant_be_moved() {
    let scene = Scene::new(vec![
      LightLayer::sequence(LightQ::name("a"), layer()).into(),
      LightLayer::sequence(LightQ::name("b"), layer()).into(),
    ]);
    assert_eq!(
      scene.moved_to_lights(LightQ::name("other")).unwrap_err(),
      DifferentLights
    );
  }

  #[test]
  fn a_sound_layer_survives_a_save() {
    let scene = Scene::new(vec![
      LightLayer::sequence(LightQ::name("lane"), layer()).into(),
      SoundLayer::new("sounds/zap.mp3")
        .start(0.25)
        .skip(0.5)
        .end(0.75)
        .name("hits")
        .into(),
    ])
    .timing(Timing::new(1.0, Cycle::Once));
    let written = scene.save().unwrap();
    assert!(written.contains("Sound("), "{written}");

    let read = Scene::load(&written).unwrap();
    let sound = read.layers[1].as_sound().unwrap();
    assert_eq!(sound.name.as_deref(), Some("hits"));
    assert_eq!(
      (sound.start, sound.skip, sound.end),
      (0.25, 0.5, Some(0.75))
    );
    assert_eq!(read.sound_names().collect::<Vec<_>>(), ["sounds/zap.mp3"]);
    assert_eq!(read.layers[1].z(), None);
  }

  #[test]
  fn a_sound_layer_with_no_volume_or_times_plays_all_of_it_as_recorded_at_the_start() {
    let scene = Scene::load(r#"Scene(version: 2, layers: [Sound(sound: "a.wav")])"#).unwrap();
    let variables = Variables::new();
    let started = scene.sounds_started(
      &Moment {
        time: 0.1,
        playback_seed: 0,
        variables: &variables,
      },
      None,
    );
    assert_eq!(
      started,
      [SoundStart {
        sound: "a.wav",
        time: 0.0,
        volume: 1.0,
        skip: 0.0,
        length: None
      }]
    );
  }

  #[test]
  fn each_sound_plays_at_its_layers_volume_in_the_order_they_start() {
    let late = SoundLayer::new("a.wav")
      .start(0.5)
      .skip(0.125)
      .end(0.75)
      .volume(0.5);
    let early = SoundLayer::new("b.wav").start(0.25).volume(0.25);
    let scene =
      Scene::new(vec![late.into(), early.into()]).timing(Timing::new(1.0, Cycle::Forever));
    let variables = Variables::new();

    let started = scene.sounds_started(
      &Moment {
        time: 1.6,
        playback_seed: 0,
        variables: &variables,
      },
      Some(1.0),
    );
    assert_eq!(
      started,
      [
        SoundStart {
          sound: "b.wav",
          time: 1.25,
          volume: 0.25,
          skip: 0.0,
          length: None
        },
        SoundStart {
          sound: "a.wav",
          time: 1.5,
          volume: 0.5,
          skip: 0.125,
          length: Some(0.25)
        },
      ]
    );
  }

  #[test]
  fn a_cycle_has_its_starts_at_their_times_in_it() {
    let scene = Scene::new(vec![
      SoundLayer::new("b.wav").start(0.5).into(),
      SoundLayer::new("a.wav").into(),
    ])
    .timing(Timing::new(2.0, Cycle::Times(2)));
    let times = |cycle| {
      scene
        .sounds_in_cycle(cycle)
        .iter()
        .map(|start| (start.sound, start.time))
        .collect::<Vec<_>>()
    };
    assert_eq!(times(1), [("a.wav", 2.0), ("b.wav", 2.5)]);
    assert_eq!(times(5), times(1), "past the last cycle is the last");
  }

  #[test]
  fn values_are_read_and_written() {
    let source = r#"Scene(version: 2, variables: {"color": Value("current")}, values: {"current": Color(Variable("lit"))}, layers: [Display(display: "Dmd", layer: (fill: (base: Solid(Variable("color")))))])"#;
    let scene = Scene::load(source).unwrap();
    assert_eq!(scene.variables["color"], Value::named("current"));
    let written = scene.save().unwrap();
    assert!(written.contains(r#"Value("current")"#), "{written}");
    let read = Scene::load(&written).unwrap();
    assert!(matches!(read.values["current"], NamedValue::Color(_)));
    assert_eq!(
      read.inputs().variables.keys().collect::<Vec<_>>(),
      ["color", "lit"]
    );
  }

  #[test]
  fn a_file_from_before_variables_says_params() {
    let scene = Scene::load(r#"Scene(version: 2, params: {"size": Float(2.0)}, layers: [Lights(surface: Sequence, layer: (fill: (base: Solid(Param("color")))))])"#).unwrap();
    assert_eq!(scene.variables["size"], Value::Float(2.0));
    assert_eq!(
      scene.inputs().variables.keys().collect::<Vec<_>>(),
      ["color"]
    );
  }

  #[test]
  fn an_animated_value_needs_a_timing() {
    let scene = Scene::load(
      r#"Scene(version: 2, values: {"pulse": Float(Keyframes((keys: [(at: 0.0, value: 0.0), (at: 1.0, value: 1.0)], cycle: Once)))})"#,
    );
    assert!(scene.is_err());
  }

  #[test]
  fn a_bundled_scene_opens_until_a_file_replaces_it() {
    let content = Content::without_folder().with_scenes([(
      "frontbox/a.scene",
      "Scene(version: 2, timing: (duration: 2.0))",
    )]);
    assert_eq!(
      Scene::open(&content, "frontbox/a.scene")
        .unwrap()
        .timing
        .unwrap()
        .duration,
      2.0
    );
    assert!(matches!(
      Scene::open(&content, "b.scene"),
      Err(OpenError::File(_))
    ));
  }

  /// A 16-bit mono WAV at 1000 frames a second, loud for its first tenth of a second and silent for the rest of a second
  fn hit() -> Vec<u8> {
    let samples: Vec<i16> = (0..1000)
      .map(|frame| if frame < 100 { i16::MAX } else { 0 })
      .collect();
    let data = (samples.len() * 2) as u32;
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + data).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend([1u16, 1].map(u16::to_le_bytes).concat()); // PCM, mono
    bytes.extend(1000u32.to_le_bytes());
    bytes.extend(2000u32.to_le_bytes());
    bytes.extend([2u16, 16].map(u16::to_le_bytes).concat());
    bytes.extend(b"data");
    bytes.extend(data.to_le_bytes());
    samples
      .iter()
      .for_each(|sample| bytes.extend(sample.to_le_bytes()));
    bytes
  }

  /// A scene whose value "glow" follows sound layer `layer`, which plays `hit.wav` half a second into each cycle
  fn following(layer: &str) -> Result<Scene, FormatError> {
    Scene::load(&format!(
      r#"Scene(
        version: 2,
        timing: (duration: 1.0, cycle: Forever),
        values: {{"glow": Float(ExternalEnvelope(source: Sound(layer: "{layer}"), spread: Float(0.0, 1.0)))}},
        layers: [Sound(name: "hit", sound: "hit.wav", start: 0.5)],
      )"#
    ))
  }

  #[test]
  fn an_envelope_follows_its_sound_where_the_layer_plays_it() {
    let folder =
      std::env::temp_dir().join(format!("frontbox-scene-envelope-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("hit.wav"), hit()).unwrap();
    let scene = following("hit")
      .unwrap()
      .with_assets(&Content::new(&folder));
    std::fs::remove_dir_all(&folder).unwrap();
    let scene = scene.unwrap();

    let NamedValue::Float(glow) = &scene.values["glow"] else {
      panic!("{:?}", scene.values);
    };
    let variables = Variables::new();
    let at = |time| {
      let moment = Moment {
        time,
        playback_seed: 0,
        variables: &variables,
      };
      let frame = scene.frame(&moment, Surface::Sequence, Vec2::ONE, None);
      (glow.at(&frame) * 100.0).round() / 100.0
    };
    assert_eq!(
      [0.2, 0.55, 0.8, 1.55].map(at),
      [0.0, 1.0, 0.0, 1.0],
      "loud only in the tenth of a second after each start"
    );
  }

  #[test]
  fn an_envelope_has_to_follow_a_sound_layer_the_scene_has() {
    if let Err(error) = following("hit") {
      panic!("{error}");
    }
    let error = following("nope").unwrap_err().to_string();
    assert!(error.contains("sound layer \"nope\""), "{error}");
  }

  #[test]
  fn an_envelopes_sound_has_to_be_in_the_content_folder() {
    let error = following("hit")
      .unwrap()
      .with_assets(&Content::without_folder())
      .unwrap_err();
    assert_eq!(error.layer, "layer \"hit\"");
  }

  #[test]
  fn a_start_after_the_end_of_a_cycle_is_refused() {
    let scene = Scene::new(vec![SoundLayer::new("a.wav").start(2.0).into()])
      .timing(Timing::new(1.0, Cycle::Once));
    assert_eq!(
      scene.check(),
      Err(SceneError::StartOutsideCycle {
        layer: "layer 1".into()
      })
    );
  }
}
