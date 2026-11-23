use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::animation::Curve;
use crate::scene::{Animatable, Env, Prop, SceneError, Spread};

/// A value that moves through its spread by one of the scene's [`Modulator`]s
///
/// The modulator sets the wave's shape and how many go by each cycle, and can be shared by many props. Each prop sets
/// its own range and phase.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Modulated {
  /// Name of the scene's modulator it moves by. A missing one gives the type's default.
  pub modulator: String,
  /// The range the wave moves through, from its low point to its high point. For a `Vec2` or a color, every part moves
  /// together; give a part a range of one value to hold it still.
  pub spread: Spread,
  /// How far the wave is shifted, 0 to 1. Moving it moves the wave along: rising by 1 over the scene adds a wave.
  #[serde(default = "no_phase")]
  pub phase: Box<Prop<f32>>,
}

impl Modulated {
  pub(crate) fn resolve<T: Animatable>(&self, env: &Env) -> T {
    let Some(modulator) = env.modulators.get(&self.modulator) else {
      return T::default();
    };
    let height = modulator.height(env, self.phase.resolve(env));
    T::from_spread(&self.spread, [height; 3]).unwrap_or_default()
  }
}

fn no_phase() -> Box<Prop<f32>> {
  Box::new(Prop::Fixed(0.0))
}

/// A scene's modulators, by name
pub type Modulators = BTreeMap<String, Modulator>;

static NO_MODULATORS: Modulators = BTreeMap::new();

/// An empty set, for a scene with no modulators
pub fn no_modulators() -> &'static Modulators {
  &NO_MODULATORS
}

/// A repeating wave timed to the scene cycle, which [`Modulated`] props move by
///
/// [`Keyframes`](crate::scene::Keyframes) are in seconds, so a wave made of keys drifts when the scene's duration
/// changes. This keeps the same number of waves per cycle at any duration.
///
/// In RON: `(envelope: (attack: (curve: Linear)), rate: 2.0)`
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Modulator {
  pub envelope: Envelope,
  /// Waves per scene cycle, at least 1. A whole number starts each cycle at the same point in the wave. A fraction
  /// carries on across cycles, so 1.5 starts every other cycle halfway through a wave.
  pub rate: f32,
}

/// The fewest waves a modulator can have per cycle, so that each cycle shows a whole one
pub const MIN_RATE: f32 = 1.0;

impl Modulator {
  pub fn new(envelope: Envelope, rate: f32) -> Self {
    Self { envelope, rate }
  }

  /// Checks that its wave goes by at least once a cycle, so every cycle shows the whole of it
  pub fn check(&self, name: &str) -> Result<(), SceneError> {
    if self.rate >= MIN_RATE {
      return Ok(());
    }
    Err(SceneError::ModulatorTooSlow {
      modulator: name.into(),
    })
  }

  /// The wave's height at `env`, shifted along by `phase`. Usually 0 to 1, though some curves overshoot.
  fn height(&self, env: &Env, phase: f32) -> f32 {
    self.envelope.sample(self.position(env, phase))
  }

  /// Where in the current wave `env` is, 0 to 1. Counts the cycles already played, so a fractional rate carries on.
  /// Without a timing, the scene never moves past its start.
  fn position(&self, env: &Env, phase: f32) -> f32 {
    let cycles = env
      .timing
      .map_or(0.0, |timing| env.cycle as f32 + env.time / timing.duration);
    (cycles * self.rate + phase).rem_euclid(1.0)
  }
}

/// The shape of one wave of a [`Modulator`]: the attack alone over the whole wave, or the attack over its first half
/// and the decay over its second
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
  pub attack: EnvelopeCurve,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub decay: Option<EnvelopeCurve>,
}

impl Envelope {
  pub fn new(attack: EnvelopeCurve) -> Self {
    Self {
      attack,
      decay: None,
    }
  }

  pub fn decay(self, decay: EnvelopeCurve) -> Self {
    Self {
      decay: Some(decay),
      ..self
    }
  }

  /// Its height at `position` through the wave, 0 to 1. Usually 0 to 1 too, though some curves overshoot.
  fn sample(&self, position: f32) -> f32 {
    match &self.decay {
      None => self.attack.sample(position),
      Some(_) if position < 0.5 => self.attack.sample(position * 2.0),
      Some(decay) => decay.sample(position * 2.0 - 1.0),
    }
  }
}

/// A curve from 0 to 1, or played backwards from 1 to 0
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeCurve {
  pub curve: Curve,
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
  pub reversed: bool,
}

impl EnvelopeCurve {
  pub fn new(curve: Curve) -> Self {
    Self {
      curve,
      reversed: false,
    }
  }

  pub fn reversed(self) -> Self {
    Self {
      reversed: true,
      ..self
    }
  }

  fn sample(&self, phase: f32) -> f32 {
    self
      .curve
      .sample(if self.reversed { 1.0 - phase } else { phase })
  }
}

#[cfg(test)]
mod tests {
  use glam::Vec2;
  use image::Rgba;

  use super::*;
  use crate::animation::Cycle;
  use crate::scene::value::tests::env;
  use crate::scene::{Color, Key, Keyframes, Timing, Value, Variables};

  const WAVE: &str = "wave";

  /// A prop moving from 10 to 20 by the modulator named `WAVE`
  fn modulated() -> Modulated {
    Modulated {
      modulator: WAVE.into(),
      spread: Spread::Float(10.0, 20.0),
      phase: Box::new(0.0.into()),
    }
  }

  /// One modulator, named `WAVE`
  fn modulators(envelope: Envelope, rate: f32) -> Modulators {
    Modulators::from([(WAVE.into(), Modulator::new(envelope, rate))])
  }

  fn rising() -> Envelope {
    Envelope::new(EnvelopeCurve::new(Curve::Linear))
  }

  fn falling() -> Envelope {
    Envelope::new(EnvelopeCurve::new(Curve::Linear).reversed())
  }

  fn up_and_down() -> Envelope {
    rising().decay(EnvelopeCurve::new(Curve::Linear).reversed())
  }

  /// Its value at a moment of a scene 4 seconds long that loops forever
  fn at(modulated: &Modulated, modulators: &Modulators, scene_time: f32) -> f32 {
    let variables = Variables::new();
    let timing = Timing::new(4.0, Cycle::Forever);
    let env = Env {
      cycle: timing.cycle_index(scene_time),
      timing: Some(timing),
      modulators,
      ..env(timing.local_time(scene_time), &variables)
    };
    modulated.resolve(&env)
  }

  fn samples(envelope: Envelope) -> [f32; 4] {
    let modulators = modulators(envelope, 1.0);
    [0.0, 1.0, 2.0, 3.0].map(|time| at(&modulated(), &modulators, time))
  }

  #[test]
  fn an_attack_alone_takes_the_whole_wave() {
    assert_eq!(samples(rising()), [10.0, 12.5, 15.0, 17.5]);
    assert_eq!(
      samples(Envelope::new(EnvelopeCurve::new(Curve::EaseIn))),
      [10.0, 10.625, 12.5, 15.625]
    );
  }

  #[test]
  fn a_reversed_curve_runs_from_high_to_low() {
    assert_eq!(samples(falling()), [20.0, 17.5, 15.0, 12.5]);
  }

  #[test]
  fn with_a_decay_each_takes_half_the_wave() {
    assert_eq!(samples(up_and_down()), [10.0, 15.0, 20.0, 15.0]);
    let falling_then_easing_in = falling().decay(EnvelopeCurve::new(Curve::EaseIn));
    assert_eq!(samples(falling_then_easing_in), [20.0, 15.0, 10.0, 12.5]);
  }

  #[test]
  fn rate_is_waves_per_cycle() {
    let twice = modulators(up_and_down(), 2.0);
    assert_eq!(
      [0.0, 1.0, 2.0, 3.0].map(|time| at(&modulated(), &twice, time)),
      [10.0, 20.0, 10.0, 20.0]
    );
  }

  #[test]
  fn a_fractional_rate_carries_on_into_the_next_cycle() {
    let one_and_a_half = modulators(rising(), 1.5);
    assert_eq!(at(&modulated(), &one_and_a_half, 0.0), 10.0);
    assert_eq!(at(&modulated(), &one_and_a_half, 4.0), 15.0);
    assert_eq!(at(&modulated(), &one_and_a_half, 6.0), 12.5);
    assert_eq!(at(&modulated(), &one_and_a_half, 8.0), 10.0);
  }

  #[test]
  fn a_rate_below_one_fails_the_check() {
    assert!(Modulator::new(rising(), 1.0).check(WAVE).is_ok());
    assert_eq!(
      Modulator::new(rising(), 0.5).check(WAVE),
      Err(SceneError::ModulatorTooSlow {
        modulator: WAVE.into()
      })
    );
    assert!(Modulator::new(rising(), f32::NAN).check(WAVE).is_err());
  }

  #[test]
  fn phase_moves_where_the_scene_starts() {
    let shifted = Modulated {
      phase: Box::new(0.5.into()),
      ..modulated()
    };
    let up_and_down = modulators(up_and_down(), 1.0);
    assert_eq!(at(&shifted, &up_and_down, 0.0), 20.0);
    assert_eq!(at(&shifted, &up_and_down, 2.0), 10.0);
    let past_one = Modulated {
      phase: Box::new(1.25.into()),
      ..modulated()
    };
    assert_eq!(at(&past_one, &modulators(rising(), 1.0), 0.0), 12.5);
  }

  #[test]
  fn props_sharing_a_modulator_keep_their_own_range_and_phase() {
    let shared = modulators(rising(), 1.0);
    let low = modulated();
    let high = Modulated {
      spread: Spread::Float(100.0, 200.0),
      phase: Box::new(0.5.into()),
      ..modulated()
    };
    assert_eq!(at(&low, &shared, 0.0), 10.0);
    assert_eq!(at(&high, &shared, 0.0), 150.0);
  }

  #[test]
  fn a_missing_modulator_is_the_default() {
    assert_eq!(at(&modulated(), no_modulators(), 1.0), 0.0);
  }

  #[test]
  fn a_phase_rising_through_the_scene_adds_waves() {
    let keys = vec![
      Key::new(0.0, 0.0, Curve::Linear),
      Key::new(4.0, 1.0, Curve::Linear),
    ];
    let phase = Prop::Keyframes(Keyframes {
      keys,
      cycle: Cycle::Once,
    });
    let faster = Modulated {
      phase: Box::new(phase),
      ..modulated()
    };
    let rising = modulators(rising(), 1.0);
    // a wave from the rate and one from the phase, so two over the scene
    assert_eq!(
      [0.0, 1.0, 2.0, 3.0].map(|time| at(&faster, &rising, time)),
      [10.0, 15.0, 10.0, 15.0]
    );
  }

  #[test]
  fn a_phase_can_be_a_param() {
    let variables = Variables::from([("offset".into(), Value::Float(0.5))]);
    let modulators = modulators(up_and_down(), 1.0);
    let shifted = Modulated {
      phase: Box::new(Prop::Variable("offset".into())),
      ..modulated()
    };
    let env = Env {
      modulators: &modulators,
      ..env(0.0, &variables)
    };
    assert_eq!(shifted.resolve::<f32>(&env), 20.0);
  }

  #[test]
  fn every_part_of_a_vec2_and_a_color_moves_together() {
    let variables = Variables::new();
    let modulators = modulators(rising(), 1.0);
    let timing = Timing::new(4.0, Cycle::Forever);
    let env = Env {
      timing: Some(timing),
      modulators: &modulators,
      ..env(2.0, &variables)
    };
    let spread = Spread::Vec2 {
      x: (0.0, 10.0),
      y: (5.0, 5.0),
    };
    let point: Vec2 = Modulated {
      spread,
      ..modulated()
    }
    .resolve(&env);
    assert_eq!(point, Vec2::new(5.0, 5.0));

    let spread = Spread::Color {
      hue: (0.0, 240.0),
      brightness: (1.0, 1.0),
      alpha: (0.0, 1.0),
    };
    let color: Color = Modulated {
      spread,
      ..modulated()
    }
    .resolve(&env);
    assert_eq!(color.0, Rgba([0, 255, 0, 128]));
  }

  #[test]
  fn with_no_timing_it_holds_its_starting_value() {
    let variables = Variables::new();
    let modulators = modulators(up_and_down(), 1.0);
    let shifted = Modulated {
      phase: Box::new(0.5.into()),
      ..modulated()
    };
    let env = Env {
      modulators: &modulators,
      ..env(3.0, &variables)
    };
    assert_eq!(shifted.resolve::<f32>(&env), 20.0);
  }

  #[test]
  fn a_spread_that_doesnt_match_the_type_is_the_default() {
    let variables = Variables::new();
    let modulators = modulators(rising(), 1.0);
    let env = Env {
      modulators: &modulators,
      ..env(0.0, &variables)
    };
    assert_eq!(modulated().resolve::<Color>(&env), Color::default());
  }

  #[test]
  fn a_prop_reads_its_wave_and_is_animated() {
    let variables = Variables::new();
    let modulators = modulators(falling(), 1.0);
    let env = Env {
      modulators: &modulators,
      ..env(0.0, &variables)
    };
    let prop: Prop<f32> = Prop::Modulated(modulated());
    assert!(prop.is_animated());
    assert_eq!(prop.resolve(&env), 20.0);
  }
}
