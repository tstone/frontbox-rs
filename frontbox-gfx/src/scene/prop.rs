use serde::{Deserialize, Serialize};

use crate::scene::{Animatable, Env, Random, Track};

/// A value in a definition that can be fixed, read from a parameter, keyframed, or picked at random
///
/// In RON: `Fixed(4.0)`, `Param("size")`, `Track((keys: [...]))`, or `Random((picks: [...], spread: Float(0.0, 1.0)))`.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Prop<T> {
  /// A value that doesn't change
  Fixed(T),
  /// Read from a parameter. A missing parameter, or one of the wrong type, gives the type's default.
  Param(String),
  /// Keyframes over time
  Track(Track<T>),
  /// A new value at set times, from within a range
  Random(Random),
}

impl<T> From<T> for Prop<T> {
  fn from(value: T) -> Self {
    Self::Fixed(value)
  }
}

impl<T> Prop<T> {
  pub(crate) fn is_animated(&self) -> bool {
    matches!(self, Self::Track(_) | Self::Random(_))
  }
}

impl<T: Animatable> Prop<T> {
  pub(crate) fn resolve(&self, env: &Env) -> T {
    match self {
      Self::Fixed(value) => value.clone(),
      Self::Param(name) => env.param(name).and_then(T::from_value).unwrap_or_default(),
      Self::Track(track) => track.resolve(env.time),
      Self::Random(random) => random.resolve(env),
    }
  }
}

impl Prop<f32> {
  /// For a value that drops its fraction where it's used, so that random picks are different once that's happened
  pub(crate) fn resolve_whole(&self, env: &Env) -> f32 {
    match self {
      Self::Random(random) => random.resolve_whole(env),
      other => other.resolve(env),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::scene::value::tests::env;
  use crate::scene::{Color, Images, Params, Value};

  #[test]
  fn fixed_resolves_to_its_value() {
    let params = Params::new();
    assert_eq!(Prop::Fixed(3.0).resolve(&env(5.0, &params)), 3.0);
  }

  #[test]
  fn param_prefers_the_games_value_over_the_default() {
    let defaults = Params::from([("size".into(), Value::Float(1.0)), ("other".into(), Value::Float(9.0))]);
    let params = Params::from([("size".into(), Value::Float(2.0))]);
    let env = Env { time: 0.0, cycle: 0, timing: None, take: 0, params: &params, defaults: &defaults, images: Images::none(), lap: None };
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
