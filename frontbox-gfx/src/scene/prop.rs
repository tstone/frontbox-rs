use serde::{Deserialize, Serialize};

use crate::scene::layer::env;
use crate::scene::sweep::only_for_positions;
use crate::scene::{Animatable, Env, Keyframes, LayerFrame, Modulated, Random, Sweep, VisitResponse};

/// A value in a definition that can be fixed, read from a parameter, keyframed, picked at random, moved in a wave, swept
/// along a path, or set off by a traversal's visits
///
/// In RON: `Fixed(4.0)`, `Param("size")`, `Keyframes((keys: [...]))`, `Random((picks: [...], spread: Float(0.0, 1.0)))`,
/// `Modulated((wave: Sine, spread: Float(0.0, 1.0), rate: 2.0))`, `Sweep((path: (base: Circle(radius: Fixed(5.0))), position: Param("lap")))`, or
/// `Visit((traversal: "bolt", from: 0.0, to: 1.0, smooth_out: 3.0))`.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "T: Animatable + Deserialize<'de>"))]
pub enum Prop<T> {
  Fixed(T),
  /// Read from a parameter. A missing parameter, or one of the wrong type, gives the type's default.
  Param(String),
  /// Keyframes over time
  Keyframes(Keyframes<T>),
  /// Picked at random at set times
  Random(Random),
  /// Moves through a range in a repeating wave, timed to the scene cycle
  Modulated(Modulated),
  /// Moves along a shape's outline. Only valid for positions (`Vec2`); a file using it for anything else fails to load.
  #[serde(deserialize_with = "only_for_positions::<T, _>")]
  Sweep(Sweep),
  /// Responds to a traversal visiting the node being drawn, on a layer with a graph
  Visit(VisitResponse<T>),
}

impl<T> From<T> for Prop<T> {
  fn from(value: T) -> Self {
    Self::Fixed(value)
  }
}

impl<T> Prop<T> {
  /// Whether it can change while a scene plays, which needs the scene to have a timing
  pub fn is_animated(&self) -> bool {
    match self {
      Self::Fixed(_) | Self::Param(_) => false,
      Self::Sweep(sweep) => sweep.is_animated(),
      Self::Keyframes(_) | Self::Random(_) | Self::Modulated(_) | Self::Visit(_) => true,
    }
  }
}

impl<T: Animatable> Prop<T> {
  /// Its value in `frame`, for a value that isn't sampled at a point, such as a sound layer's volume. A `Sweep` or
  /// `Visit` has no point to go on, so it gives what it would at the frame's origin with nothing visited.
  pub fn at(&self, frame: &LayerFrame) -> T {
    self.resolve(&env(frame))
  }

  pub(crate) fn resolve(&self, env: &Env) -> T {
    match self {
      Self::Fixed(value) => value.clone(),
      Self::Param(name) => env.param(name).and_then(T::from_value).unwrap_or_default(),
      Self::Keyframes(keyframes) => keyframes.resolve(env.time),
      Self::Random(random) => random.resolve(env),
      Self::Modulated(modulated) => modulated.resolve(env),
      Self::Sweep(sweep) => T::from_point(sweep.resolve(env)).unwrap_or_default(),
      Self::Visit(response) => response.resolve(env),
    }
  }
}

impl Prop<f32> {
  /// Like `resolve`, for values that are truncated to a whole number where they're used (blade counts, frame numbers).
  /// For a `Random`, this makes `force_unique` compare whole numbers, so consecutive picks don't truncate to the same one.
  pub(crate) fn resolve_truncated(&self, env: &Env) -> f32 {
    match self {
      Self::Random(random) => random.resolve_truncated(env),
      other => other.resolve(env),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::scene::value::tests::env;
  use crate::scene::{Bounds, Color, Images, Params, Value};

  #[test]
  fn fixed_resolves_to_its_value() {
    let params = Params::new();
    assert_eq!(Prop::Fixed(3.0).resolve(&env(5.0, &params)), 3.0);
  }

  #[test]
  fn param_prefers_the_games_value_over_the_default() {
    let defaults = Params::from([("size".into(), Value::Float(1.0)), ("other".into(), Value::Float(9.0))]);
    let params = Params::from([("size".into(), Value::Float(2.0))]);
    let env = Env { time: 0.0, cycle: 0, timing: None, playback_seed: 0, params: &params, defaults: &defaults, images: Images::none(), viewport: Bounds::sized(glam::Vec2::ONE), lap: None, visits: None, fill_frame: glam::Affine2::IDENTITY };
    assert_eq!(Prop::<f32>::Param("size".into()).resolve(&env), 2.0);
    assert_eq!(Prop::<f32>::Param("other".into()).resolve(&env), 9.0);
  }

  #[test]
  fn param_that_is_missing_or_the_wrong_type_is_the_default() {
    let params = Params::from([("size".into(), Value::Color(Color::default()))]);
    assert_eq!(Prop::<f32>::Param("size".into()).resolve(&env(0.0, &params)), 0.0);
    assert_eq!(Prop::<f32>::Param("nope".into()).resolve(&env(0.0, &params)), 0.0);
  }
}
