use std::fmt;

use frontbox_core::prelude::*;
use frontbox_gfx::animation::Cycle;
use frontbox_gfx::scene::file::{self, FormatError};
use frontbox_gfx::scene::{self, Content, Images, LayerFrame, Layer, LayerImageError, Params, SceneError, Timing, Visits};
use serde::{Deserialize, Serialize};

use crate::{Moment, SoundLayer, SoundStart};

/// Layers on one timeline, each saying what it's drawn on or that it's sound, as written to a `.scene` file. Each system
/// plays the layers it knows: `LedSystem` plays [`LedLayer`]s, a display plays the [`DisplayLayer`]s for it, and
/// `SoundSystem` plays [`SoundLayer`]s. A [`Playback`](crate::Playback) holds what's only true while one plays.
///
/// ```rust
/// # use frontbox_scene::*;
/// let scene = Scene::load(r#"Scene(
///   version: 2,
///   timing: (duration: 0.5),
///   layers: [
///     Leds(leds: Tag("Playfield"), z: 1, surface: Sequence, layer: (fill: (base: Solid(Fixed(255, 0, 0, 255))))),
///     Sound(sound: "sounds/zap.mp3"),
///   ],
/// )"#).unwrap();
/// ```
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
  pub version: u32,
  /// Parameter names and their defaults
  #[serde(default, skip_serializing_if = "Params::is_empty")]
  #[cfg_attr(feature = "ts", ts(as = "Option<Params>", optional))]
  pub params: Params,
  /// `None` is static, and its tracks stay on their first moment
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub timing: Option<Timing>,
  /// Drawn bottom to top where they land on the same thing at the same z
  #[serde(default)]
  pub layers: Vec<SceneLayer>,
  /// The decoded images its layers draw, from [`Scene::with_images`]. They aren't written to the file, which only names
  /// them.
  #[serde(skip)]
  #[cfg_attr(feature = "ts", ts(skip))]
  pub images: Images,
}

impl Scene {
  /// A scene of the current format version
  pub fn new(layers: Vec<SceneLayer>) -> Self {
    Self { version: file::VERSION, params: Params::new(), timing: None, layers, images: Images::new() }
  }

  pub fn timing(mut self, timing: Timing) -> Self {
    self.timing = Some(timing);
    self
  }

  /// Reads a `.scene` file, which has to be one that can be played
  pub fn load(source: &str) -> Result<Self, FormatError> {
    let SceneFile::Scene(scene) = file::load(source)?;
    file::check_version(scene.version)?;
    scene.check().map_err(|err| FormatError::Invalid(err.to_string()))?;
    Ok(scene)
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
    self.layers.iter().enumerate().try_for_each(|(index, layer)| match layer {
      SceneLayer::Sound(sound) => sound.check(index, self.timing),
      drawn => drawn.layer().map_or(Ok(()), |layer| layer.check(index, self.timing)),
    })
  }

  /// Names of the images it uses, possibly with repeats
  pub fn image_names(&self) -> impl Iterator<Item = &str> {
    self.layers.iter().filter_map(SceneLayer::layer).flat_map(Layer::image_names)
  }

  /// Names of the sounds its layers play, possibly with repeats
  pub fn sound_names(&self) -> impl Iterator<Item = &str> {
    self.layers.iter().filter_map(SceneLayer::as_sound).map(|layer| layer.sound.as_str())
  }

  /// The sounds that start after `after` and up to `moment`'s time, in the order they start, each at the volume its
  /// layer gives then. Pass the time of the last call as `after`, so each start is given once, or `None` for a playback that
  /// has just started.
  pub fn sounds_started<'a>(&'a self, moment: &Moment<'a>, after: Option<f32>) -> Vec<SoundStart<'a>> {
    let layers = self.layers.iter().filter_map(SceneLayer::as_sound);
    let starts = layers.filter_map(|layer| layer.started(self.timing, after, moment.time).map(|start| (layer, start)));
    self.sound_starts(moment, starts)
  }

  /// Where each sound starts in cycle `cycle` (counting from zero) at the volume its layer gives then, in the order they
  /// start, for previewing a cycle. `moment`'s time is ignored. A cycle past the last is the last.
  pub fn sounds_in_cycle<'a>(&'a self, moment: &Moment<'a>, cycle: u32) -> Vec<SoundStart<'a>> {
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
    self.sound_starts(moment, starts)
  }

  fn sound_starts<'a>(&'a self, moment: &Moment<'a>, starts: impl Iterator<Item = (&'a SoundLayer, f32)>) -> Vec<SoundStart<'a>> {
    let mut started: Vec<_> = starts
      .map(|(layer, time)| {
        let frame = self.frame(&Moment { time, ..*moment }, scene::Surface::Sequence, Vec2::ZERO, None);
        SoundStart { sound: &layer.sound, time, volume: layer.volume_at(&frame), skip: layer.skip, length: layer.length() }
      })
      .collect();
    started.sort_by(|a, b| a.time.total_cmp(&b.time));
    started
  }

  /// Loads the images its layers draw from `content`. A layer whose image is missing is an error, rather than one that
  /// silently draws nothing. That reads files, so do it ahead of time rather than in a tick.
  pub fn with_images(mut self, content: &Content) -> Result<Self, LayerImageError> {
    self.images = content.images_for(self.layers.iter().filter_map(SceneLayer::layer))?;
    Ok(self)
  }

  /// Whether every cycle has played by `time`. A scene that's static or repeats forever never completes.
  pub fn is_complete(&self, time: f32) -> bool {
    self.timing.is_some_and(|timing| timing.is_complete(time))
  }

  /// Moves every LED layer to `leds`, for when which LEDs a scene plays on is only known in the game. Each layer's z and
  /// surface stay as the scene says. That's only for a scene whose LED layers are all on the same LEDs, as there's
  /// otherwise no saying which of them to move.
  pub fn moved_to_leds(mut self, leds: LedQ) -> Result<Self, DifferentLeds> {
    let mut on = self.layers.iter().filter_map(SceneLayer::as_leds).map(|layer| &layer.leds);
    if let Some(first) = on.next()
      && on.any(|other| other != first)
    {
      return Err(DifferentLeds);
    }
    for layer in self.layers.iter_mut().filter_map(SceneLayer::as_leds_mut) {
      layer.leds = Some(leds.clone());
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
      params: moment.params,
      defaults: &self.params,
      images: &self.images,
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
  Leds(LedLayer),
  Display(DisplayLayer),
  Sound(SoundLayer),
}

impl SceneLayer {
  /// What's drawn, for an LED or display layer
  pub fn layer(&self) -> Option<&Layer> {
    match self {
      Self::Leds(layer) => Some(&layer.layer),
      Self::Display(layer) => Some(&layer.layer),
      Self::Sound(_) => None,
    }
  }

  pub fn layer_mut(&mut self) -> Option<&mut Layer> {
    match self {
      Self::Leds(layer) => Some(&mut layer.layer),
      Self::Display(layer) => Some(&mut layer.layer),
      Self::Sound(_) => None,
    }
  }

  /// Order among everything playing on the same thing: higher is on top. `None` for a sound layer.
  pub fn z(&self) -> Option<i8> {
    match self {
      Self::Leds(layer) => Some(layer.z),
      Self::Display(layer) => Some(layer.z),
      Self::Sound(_) => None,
    }
  }

  /// How logs and errors refer to the layer at `index`
  pub fn label(&self, index: usize) -> String {
    match self {
      Self::Sound(layer) => layer.label(index),
      drawn => drawn.layer().map(|layer| layer.label(index)).unwrap_or_default(),
    }
  }

  pub fn as_leds(&self) -> Option<&LedLayer> {
    match self {
      Self::Leds(layer) => Some(layer),
      _ => None,
    }
  }

  pub fn as_leds_mut(&mut self) -> Option<&mut LedLayer> {
    match self {
      Self::Leds(layer) => Some(layer),
      _ => None,
    }
  }

  pub fn as_display(&self) -> Option<&DisplayLayer> {
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

impl From<LedLayer> for SceneLayer {
  fn from(layer: LedLayer) -> Self {
    Self::Leds(layer)
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

/// A layer drawn on LEDs
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedLayer {
  /// `None` is none yet. It lands on no LEDs, and the editor previews it on every LED.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub leds: Option<LedQ>,
  /// Which layer of the LED system it's on
  #[serde(default, skip_serializing_if = "is_zero")]
  #[cfg_attr(feature = "ts", ts(as = "Option<i8>", optional))]
  pub z: i8,
  pub surface: LedSurface,
  pub layer: Layer,
}

impl LedLayer {
  /// Spreads along `leds` in order
  pub fn sequence(leds: LedQ, layer: Layer) -> Self {
    Self { leds: Some(leds), z: 0, surface: LedSurface::Sequence, layer }
  }

  /// Covers a plane, by the name it was added to the app under, lighting `leds` from where they are on it
  pub fn area(leds: LedQ, plane: impl Into<String>, layer: Layer) -> Self {
    Self { leds: Some(leds), z: 0, surface: LedSurface::Area(plane.into()), layer }
  }

  pub fn at_z(mut self, z: i8) -> Self {
    self.z = z;
    self
  }
}

/// What an LED layer is drawn on, which is what its numbers are measured against
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LedSurface {
  /// Its LEDs in order
  Sequence,
  /// A plane, by the name it was added to the app under, which gives the layer its size and the LEDs their positions
  Area(String),
}

impl LedSurface {
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
    Self { display: display.into(), z: 0, layer }
  }

  pub fn at_z(mut self, z: i8) -> Self {
    self.z = z;
    self
  }
}

/// Why [`Scene::moved_to_leds`] refused: the scene's LED layers aren't all on the same LEDs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifferentLeds;

impl fmt::Display for DifferentLeds {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "a scene with layers on different LEDs can't be moved to other LEDs")
  }
}

impl std::error::Error for DifferentLeds {}

fn is_zero(z: &i8) -> bool {
  *z == 0
}

/// How a file says it's a scene
#[derive(Serialize, Deserialize)]
enum SceneFile {
  Scene(Scene),
}

#[cfg(test)]
mod tests {
  use frontbox_gfx::animation::Curve;
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
    assert!(matches!(result, Err(FormatError::Newer { version: 99 })), "{result:?}");
  }

  #[test]
  fn a_layer_with_no_leds_says_so() {
    let source = "Scene(version: 2, layers: [Leds(surface: Sequence, layer: (fill: (base: Solid(Fixed(255, 0, 0, 255)))))])";
    assert_eq!(Scene::load(source).unwrap().layers[0].as_leds().unwrap().leds, None);
  }

  #[test]
  fn layers_survive_a_save_in_order() {
    let scene = Scene::new(vec![
      LedLayer::sequence(LedQ::names(["b", "a"]).reverse(), layer().name("under")).at_z(-1).into(),
      LedLayer::area(LedQ::Every, "Backbox", layer().name("over")).into(),
      DisplayLayer::new("Dmd", layer().name("screen")).at_z(2).into(),
    ]);
    let read = Scene::load(&scene.save().unwrap()).unwrap();
    let names: Vec<_> = read.layers.iter().map(|layer| layer.layer().unwrap().name.clone().unwrap()).collect();
    assert_eq!(names, ["under", "over", "screen"]);

    let under = read.layers[0].as_leds().unwrap();
    assert_eq!((&under.leds, under.z, &under.surface), (&scene.layers[0].as_leds().unwrap().leds, -1, &LedSurface::Sequence));
    assert_eq!(read.layers[1].as_leds().unwrap().surface, LedSurface::Area("Backbox".into()));
    let screen = read.layers[2].as_display().unwrap();
    assert_eq!((screen.display.as_str(), screen.z), ("Dmd", 2));
  }

  #[test]
  fn a_layer_says_what_its_on() {
    let written = Scene::new(vec![
      LedLayer::area(LedQ::tag::<frontbox_core::tags::Playfield>(), "Playfield", layer()).into(),
      DisplayLayer::new("Dmd", layer()).into(),
    ])
    .save()
    .unwrap();
    assert!(written.contains(r#"Leds("#), "{written}");
    assert!(written.contains(r#"Tag("Playfield")"#), "{written}");
    assert!(written.contains(r#"surface: Area("Playfield")"#), "{written}");
    assert!(written.contains(r#"Display("#), "{written}");
    assert!(written.contains(r#"display: "Dmd""#), "{written}");
  }

  #[test]
  fn a_surface_says_how_a_layer_is_seen() {
    assert_eq!(LedSurface::Sequence.sampled_as(), scene::Surface::Sequence);
    assert_eq!(LedSurface::Sequence.plane(), None);
    assert_eq!(LedSurface::Area("Backbox".into()).sampled_as(), scene::Surface::Area);
    assert_eq!(LedSurface::Area("Backbox".into()).plane(), Some("Backbox"));
  }

  #[test]
  fn what_a_layer_cant_play_is_refused_by_its_place_in_the_scene() {
    let track = Prop::Track(Track { keys: vec![], cycle: Cycle::Once });
    let changing = Fill::new(FillNode::Solid(Color(Rgba::red()).into())).adjust(ColorAdjust::HueShift(track));
    let scene = Scene::new(vec![DisplayLayer::new("Dmd", layer()).into(), LedLayer::sequence(LedQ::Every, Layer::new(changing)).into()]);
    assert_eq!(scene.check(), Err(SceneError::NoTiming { layer: "layer 2".into() }));
  }

  #[test]
  fn a_timing_with_no_length_is_refused() {
    let scene = Scene::new(vec![]).timing(Timing::new(0.0, Cycle::Once));
    assert_eq!(scene.check(), Err(SceneError::NoDuration));
  }

  #[test]
  fn led_layers_on_the_same_leds_can_be_moved_together() {
    let scene = Scene::new(vec![
      LedLayer::sequence(LedQ::name("lane"), layer()).into(),
      DisplayLayer::new("Dmd", layer()).into(),
      LedLayer::area(LedQ::name("lane"), "Playfield", layer()).at_z(1).into(),
    ]);
    let moved = scene.moved_to_leds(LedQ::name("other")).unwrap();
    let leds: Vec<_> = moved.layers.iter().filter_map(SceneLayer::as_leds).map(|layer| (layer.leds.clone(), layer.z)).collect();
    assert_eq!(leds, [(Some(LedQ::name("other")), 0), (Some(LedQ::name("other")), 1)]);
  }

  #[test]
  fn led_layers_on_different_leds_cant_be_moved() {
    let scene = Scene::new(vec![LedLayer::sequence(LedQ::name("a"), layer()).into(), LedLayer::sequence(LedQ::name("b"), layer()).into()]);
    assert_eq!(scene.moved_to_leds(LedQ::name("other")).unwrap_err(), DifferentLeds);
  }

  #[test]
  fn a_sound_layer_survives_a_save() {
    let scene = Scene::new(vec![
      LedLayer::sequence(LedQ::name("lane"), layer()).into(),
      SoundLayer::new("sounds/zap.mp3").start(0.25).skip(0.5).end(0.75).name("hits").into(),
    ])
    .timing(Timing::new(1.0, Cycle::Once));
    let written = scene.save().unwrap();
    assert!(written.contains("Sound("), "{written}");

    let read = Scene::load(&written).unwrap();
    let sound = read.layers[1].as_sound().unwrap();
    assert_eq!(sound.name.as_deref(), Some("hits"));
    assert_eq!((sound.start, sound.skip, sound.end), (0.25, 0.5, Some(0.75)));
    assert_eq!(read.sound_names().collect::<Vec<_>>(), ["sounds/zap.mp3"]);
    assert_eq!(read.layers[1].z(), None);
  }

  #[test]
  fn a_sound_layer_with_no_volume_or_times_plays_all_of_it_as_recorded_at_the_start() {
    let scene = Scene::load(r#"Scene(version: 2, layers: [Sound(sound: "a.wav")])"#).unwrap();
    let params = Params::new();
    let started = scene.sounds_started(&Moment { time: 0.1, playback_seed: 0, params: &params }, None);
    assert_eq!(started, [SoundStart { sound: "a.wav", time: 0.0, volume: 1.0, skip: 0.0, length: None }]);
  }

  #[test]
  fn each_sound_plays_at_the_volume_its_layer_has_then_in_the_order_they_start() {
    let fade_in = || Prop::Track(Track { keys: vec![Key::new(0.0, 0.0, Curve::Linear), Key::new(1.0, 1.0, Curve::Linear)], cycle: Cycle::Once });
    let late = SoundLayer::new("a.wav").start(0.5).skip(0.125).end(0.75).volume(fade_in());
    let early = SoundLayer::new("b.wav").start(0.25).volume(fade_in());
    let scene = Scene::new(vec![late.into(), early.into()]).timing(Timing::new(1.0, Cycle::Forever));
    let params = Params::new();

    let started = scene.sounds_started(&Moment { time: 1.6, playback_seed: 0, params: &params }, Some(1.0));
    assert_eq!(
      started,
      [
        SoundStart { sound: "b.wav", time: 1.25, volume: 0.25, skip: 0.0, length: None },
        SoundStart { sound: "a.wav", time: 1.5, volume: 0.5, skip: 0.125, length: Some(0.25) },
      ]
    );
  }

  #[test]
  fn a_cycle_has_its_starts_at_their_times_in_it() {
    let scene = Scene::new(vec![SoundLayer::new("b.wav").start(0.5).into(), SoundLayer::new("a.wav").into()])
      .timing(Timing::new(2.0, Cycle::Times(2)));
    let params = Params::new();
    let moment = Moment { time: 0.0, playback_seed: 0, params: &params };
    let times = |cycle| scene.sounds_in_cycle(&moment, cycle).iter().map(|start| (start.sound, start.time)).collect::<Vec<_>>();
    assert_eq!(times(1), [("a.wav", 2.0), ("b.wav", 2.5)]);
    assert_eq!(times(5), times(1), "past the last cycle is the last");
  }

  #[test]
  fn a_start_after_the_end_of_a_cycle_is_refused() {
    let scene = Scene::new(vec![SoundLayer::new("a.wav").start(2.0).into()]).timing(Timing::new(1.0, Cycle::Once));
    assert_eq!(scene.check(), Err(SceneError::StartOutsideCycle { layer: "layer 1".into() }));
  }
}
