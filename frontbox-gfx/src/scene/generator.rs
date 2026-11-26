use std::fmt;
use std::sync::{Arc, Mutex};

/// The moment a generator makes its output for, and its settings there
///
/// A generator is the app's code that a scene names and sets the settings of: a
/// [`ShapeGenerator`](crate::scene::ShapeGenerator) makes an outline for a region, and an
/// [`ImageGenerator`](crate::scene::ImageGenerator) makes an image for a fill. Both are given one of these, and have to
/// be a pure function of it.
#[derive(Debug, Clone, Copy)]
pub struct GeneratorFrame<'a> {
  /// Seconds into the current scene cycle. 0 for a generator that isn't animated.
  pub time: f32,
  /// Index of the current scene cycle, from 0. 0 for a generator that isn't animated.
  pub cycle: u32,
  /// Changes each time a scene restarts, for a generator that picks at random
  pub seed: u32,
  specs: &'a [GeneratorSetting],
  values: &'a [f32],
}

impl<'a> GeneratorFrame<'a> {
  /// A frame for calling a generator on its own, as its tests do, with each of its settings `specs` at the one of
  /// `values` in the same place. A scene's frames are made for it as it plays.
  pub fn new(
    time: f32,
    cycle: u32,
    seed: u32,
    specs: &'a [GeneratorSetting],
    values: &'a [f32],
  ) -> Self {
    debug_assert_eq!(specs.len(), values.len(), "a value for each setting");
    Self {
      time,
      cycle,
      seed,
      specs,
      values,
    }
  }

  /// The setting named `name`, within its range. 0 for a name the generator doesn't list.
  pub fn setting(&self, name: &str) -> f32 {
    let at = self.specs.iter().position(|spec| spec.name == name);
    debug_assert!(at.is_some(), "no generator setting \"{name}\"");
    at.and_then(|at| self.values.get(at))
      .copied()
      .unwrap_or(0.0)
  }
}

/// One setting of a generator, which a scene sets as a `Prop<f32>`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneratorSetting {
  pub name: &'static str,
  /// What it is when a scene leaves it out
  pub default: f32,
  /// The lowest and highest it can be. A scene's value outside it is clamped.
  pub range: Option<(f32, f32)>,
}

impl GeneratorSetting {
  pub const fn new(name: &'static str, default: f32) -> Self {
    Self {
      name,
      default,
      range: None,
    }
  }

  pub const fn range(mut self, min: f32, max: f32) -> Self {
    self.range = Some((min, max));
    self
  }

  pub(crate) fn clamped(&self, value: f32) -> f32 {
    match self.range {
      Some((min, max)) => value.clamp(min, max),
      None => value,
    }
  }
}

/// A point in a [`ShapeGenerator`](crate::scene::ShapeGenerator)'s local space that two of its settings make, such as
/// where it reaches to, which an editor shows a handle on: dragging it sets both
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneratorHandle {
  /// What an editor calls it
  pub name: &'static str,
  /// The name of the setting that's the point's x
  pub x: &'static str,
  /// The name of the setting that's the point's y
  pub y: &'static str,
}

impl GeneratorHandle {
  pub const fn new(name: &'static str, x: &'static str, y: &'static str) -> Self {
    Self { name, x, y }
  }
}

/// What a generator makes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratorKind {
  /// An outline, as a [`ShapeGenerator`](crate::scene::ShapeGenerator) does
  Shape,
  /// An image, as an [`ImageGenerator`](crate::scene::ImageGenerator) does
  Image,
}

impl fmt::Display for GeneratorKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      Self::Shape => "shape",
      Self::Image => "image",
    })
  }
}

/// Why a scene can't use a generator as it does
#[derive(Debug, Clone, PartialEq)]
pub enum GeneratorError {
  /// A scene sets a setting its generator doesn't have
  NoSuchSetting {
    kind: GeneratorKind,
    generator: String,
    setting: String,
  },
  /// A sweep follows the region of the layer named `path`, made by a shape generator with no path to follow
  NoPath { path: String, generator: String },
}

impl fmt::Display for GeneratorError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NoSuchSetting {
        kind,
        generator,
        setting,
      } => write!(
        f,
        "{kind} generator \"{generator}\" has no setting \"{setting}\""
      ),
      Self::NoPath { path, generator } => write!(
        f,
        "layer \"{path}\" is made by shape generator \"{generator}\", which has no path to follow"
      ),
    }
  }
}

impl std::error::Error for GeneratorError {}

/// A layer uses a generator in a way it can't be
#[derive(Debug, Clone, PartialEq)]
pub struct LayerGeneratorError {
  pub layer: String,
  pub error: GeneratorError,
}

impl fmt::Display for LayerGeneratorError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}: {}", self.layer, self.error)
  }
}

impl std::error::Error for LayerGeneratorError {}

/// The first of the settings a scene `sets` that isn't among a generator's `specs`
pub(crate) fn unknown_setting<'a>(
  specs: &[GeneratorSetting],
  mut sets: impl Iterator<Item = &'a str>,
) -> Option<&'a str> {
  sets.find(|setting| !specs.iter().any(|spec| spec.name == *setting))
}

/// When a generator's output is asked for
#[derive(Debug, Clone, Copy)]
pub(crate) struct Moment {
  pub time: f32,
  pub cycle: u32,
  pub seed: u32,
  /// Whether the generator's output changes with time when its settings don't. One that doesn't is asked for its
  /// output at time 0 of cycle 0, whenever this is.
  pub animated: bool,
}

/// A generator's latest outputs, kept so that every sample of a frame, and every layer with the same settings, shares
/// one call. Clones share what's kept.
#[derive(Debug)]
pub(crate) struct Kept<T> {
  outputs: Arc<Mutex<Vec<KeptOutput<T>>>>,
}

impl<T> Kept<T> {
  /// Outputs kept: enough for the layers of a scene sharing one generator with different settings
  const KEPT: usize = 16;

  /// The output at `moment` of a generator with the settings `specs`, where `setting` gives the scene's value for a
  /// setting it sets. `make` makes it, unless it's kept from being asked for before.
  pub(crate) fn output(
    &self,
    specs: &[GeneratorSetting],
    moment: Moment,
    setting: impl Fn(&str) -> Option<f32>,
    make: impl FnOnce(&GeneratorFrame) -> T,
  ) -> Arc<T> {
    let values: Vec<f32> = specs
      .iter()
      .map(|spec| spec.clamped(setting(spec.name).unwrap_or(spec.default)))
      .collect();
    let (time, cycle) = if moment.animated {
      (moment.time, moment.cycle)
    } else {
      (0.0, 0)
    };
    let key = KeptKey {
      time: time.to_bits(),
      cycle,
      seed: moment.seed,
      values: values.iter().map(|value| value.to_bits()).collect(),
    };
    if let Some((_, output)) = self
      .outputs
      .lock()
      .unwrap()
      .iter()
      .find(|(kept, _)| *kept == key)
    {
      return output.clone();
    }
    let frame = GeneratorFrame::new(time, cycle, moment.seed, specs, &values);
    let output = Arc::new(make(&frame));
    let mut outputs = self.outputs.lock().unwrap();
    if outputs.len() >= Self::KEPT {
      outputs.remove(0);
    }
    outputs.push((key, output.clone()));
    output
  }
}

impl<T> Default for Kept<T> {
  fn default() -> Self {
    Self {
      outputs: Arc::default(),
    }
  }
}

impl<T> Clone for Kept<T> {
  fn clone(&self) -> Self {
    Self {
      outputs: self.outputs.clone(),
    }
  }
}

/// An output a [`Kept`] keeps, by what it was made for
type KeptOutput<T> = (KeptKey, Arc<T>);

/// What a [`Kept`] output is kept by: the moment and resolved settings it was made for, as bits so they compare exactly
#[derive(Debug, Clone, PartialEq, Eq)]
struct KeptKey {
  time: u32,
  cycle: u32,
  seed: u32,
  values: Vec<u32>,
}
