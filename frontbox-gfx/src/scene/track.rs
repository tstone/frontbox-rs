use serde::{Deserialize, Serialize};

use crate::animation::{Curve, Cycle};
use crate::scene::Animatable;

/// Keyframes, played from time zero
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track<T> {
  /// In order of `at`
  pub keys: Vec<Key<T>>,
  /// How many times to play the keys. A track that loops ends at its last key and jumps back to its first.
  #[serde(default)]
  pub cycle: Cycle,
}

impl<T: Animatable> Track<T> {
  pub(crate) fn resolve(&self, time: f32) -> T {
    let (Some(first), Some(last)) = (self.keys.first(), self.keys.last()) else {
      return T::default();
    };

    let time = self.cycle.local_time(time, last.at);
    if time <= first.at {
      return first.value.clone();
    }

    for pair in self.keys.windows(2) {
      let (a, b) = (&pair[0], &pair[1]);
      if time < b.at {
        let span = b.at - a.at;
        let phase = if span > 0.0 { (time - a.at) / span } else { 1.0 };
        return a.value.interpolate(&b.value, b.curve.sample(phase));
      }
    }

    last.value.clone()
  }
}

/// A value at a point in time
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Key<T> {
  /// Seconds
  pub at: f32,
  pub value: T,
  /// How to get here from the key before. The first key has none before it, so this does nothing there.
  #[serde(default)]
  pub curve: Curve,
}

impl<T> Key<T> {
  pub fn new(at: f32, value: T, curve: Curve) -> Self {
    Self { at, value, curve }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::scene::value::tests::env;
  use crate::scene::{Params, Prop};

  fn track(cycle: Cycle) -> Prop<f32> {
    Prop::Track(Track {
      keys: vec![
        Key::new(0.0, 0.0, Curve::Linear),
        Key::new(1.0, 10.0, Curve::Linear),
        Key::new(2.0, 20.0, Curve::EaseIn),
      ],
      cycle,
    })
  }

  #[test]
  fn track_eases_with_the_curve_of_the_key_it_arrives_at() {
    let params = Params::new();
    let prop = track(Cycle::Once);
    assert_eq!(prop.resolve(&env(0.5, &params)), 5.0);
    // EaseIn at phase 0.5 is 0.25
    assert_eq!(prop.resolve(&env(1.5, &params)), 12.5);
  }

  #[test]
  fn the_first_keys_curve_does_nothing() {
    let params = Params::new();
    let prop = Prop::Track(Track {
      keys: vec![Key::new(0.0, 0.0, Curve::EaseIn), Key::new(1.0, 10.0, Curve::Linear)],
      cycle: Cycle::Forever,
    });
    assert_eq!(prop.resolve(&env(0.5, &params)), 5.0);
    assert_eq!(prop.resolve(&env(1.5, &params)), 5.0);
  }

  #[test]
  fn track_once_holds_its_last_key() {
    let params = Params::new();
    assert_eq!(track(Cycle::Once).resolve(&env(-1.0, &params)), 0.0);
    assert_eq!(track(Cycle::Once).resolve(&env(10.0, &params)), 20.0);
  }

  #[test]
  fn track_loops() {
    let params = Params::new();
    assert_eq!(track(Cycle::Forever).resolve(&env(4.5, &params)), 5.0);
    assert_eq!(track(Cycle::Times(2)).resolve(&env(2.5, &params)), 5.0);
    assert_eq!(track(Cycle::Times(2)).resolve(&env(4.5, &params)), 20.0);
  }

  #[test]
  fn empty_track_is_the_default() {
    let params = Params::new();
    let prop: Prop<f32> = Prop::Track(Track { keys: vec![], cycle: Cycle::Once });
    assert_eq!(prop.resolve(&env(1.0, &params)), 0.0);
  }
}
