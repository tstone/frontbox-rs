use serde::{Deserialize, Serialize};

use crate::animation::Curve;
use crate::scene::{Animatable, Env, Prop, Spread};

/// A value that moves through its spread in a repeating wave, timed to the scene cycle
///
/// [`Keyframes`](crate::scene::Keyframes) are in seconds, so a wave made of keys drifts when the scene's duration
/// changes. This keeps the same number of waves per cycle at any duration.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Modulated {
  pub envelope: Envelope,
  /// The range the wave moves through, from its low point to its high point. For a `Vec2` or a color, every part moves
  /// together; give a part a range of one value to hold it still.
  pub spread: Spread,
  /// Waves per scene cycle. A whole number starts each cycle at the same point in the wave. A fraction carries on
  /// across cycles, so 0.5 is one wave over two cycles.
  pub rate: f32,
  /// How far the wave is shifted, 0 to 1. Moving it moves the wave along: rising by 1 over the scene adds a wave.
  #[serde(default = "no_phase")]
  pub phase: Box<Prop<f32>>,
}

impl Modulated {
  pub(crate) fn resolve<T: Animatable>(&self, env: &Env) -> T {
    let height = self.envelope.sample(self.position(env));
    T::from_spread(&self.spread, [height; 3]).unwrap_or_default()
  }

  /// Where in the current wave `env` is, 0 to 1. Counts the cycles already played, so a fractional rate carries on.
  /// Without a timing, the scene never moves past its start.
  fn position(&self, env: &Env) -> f32 {
    let cycles = env.timing.map_or(0.0, |timing| env.cycle as f32 + env.time / timing.duration);
    (cycles * self.rate + self.phase.resolve(env)).rem_euclid(1.0)
  }
}

fn no_phase() -> Box<Prop<f32>> {
  Box::new(Prop::Fixed(0.0))
}

/// The shape of one wave of a [`Modulated`]: the attack alone over the whole wave, or the attack over its first half
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
    Self { attack, decay: None }
  }

  pub fn decay(self, decay: EnvelopeCurve) -> Self {
    Self { decay: Some(decay), ..self }
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
    Self { curve, reversed: false }
  }

  pub fn reversed(self) -> Self {
    Self { reversed: true, ..self }
  }

  fn sample(&self, phase: f32) -> f32 {
    self.curve.sample(if self.reversed { 1.0 - phase } else { phase })
  }
}

#[cfg(test)]
mod tests {
  use glam::Vec2;
  use image::Rgba;

  use super::*;
  use crate::animation::Cycle;
  use crate::scene::value::tests::env;
  use crate::scene::{Color, Key, Keyframes, Params, Timing, Value};

  fn modulated(envelope: Envelope) -> Modulated {
    Modulated { envelope, spread: Spread::Float(10.0, 20.0), rate: 1.0, phase: Box::new(0.0.into()) }
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
  fn at(modulated: &Modulated, scene_time: f32) -> f32 {
    let params = Params::new();
    let timing = Timing::new(4.0, Cycle::Forever);
    let env = Env { cycle: timing.cycle_index(scene_time), timing: Some(timing), ..env(timing.local_time(scene_time), &params) };
    modulated.resolve(&env)
  }

  fn samples(envelope: Envelope) -> [f32; 4] {
    [0.0, 1.0, 2.0, 3.0].map(|time| at(&modulated(envelope.clone()), time))
  }

  #[test]
  fn an_attack_alone_takes_the_whole_wave() {
    assert_eq!(samples(rising()), [10.0, 12.5, 15.0, 17.5]);
    assert_eq!(samples(Envelope::new(EnvelopeCurve::new(Curve::EaseIn))), [10.0, 10.625, 12.5, 15.625]);
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
    let twice = Modulated { rate: 2.0, ..modulated(up_and_down()) };
    assert_eq!([0.0, 1.0, 2.0, 3.0].map(|time| at(&twice, time)), [10.0, 20.0, 10.0, 20.0]);
  }

  #[test]
  fn a_fractional_rate_carries_on_into_the_next_cycle() {
    let half = Modulated { rate: 0.5, ..modulated(rising()) };
    assert_eq!(at(&half, 0.0), 10.0);
    assert_eq!(at(&half, 4.0), 15.0);
    assert_eq!(at(&half, 6.0), 17.5);
    assert_eq!(at(&half, 8.0), 10.0);
  }

  #[test]
  fn phase_moves_where_the_scene_starts() {
    let shifted = Modulated { phase: Box::new(0.5.into()), ..modulated(up_and_down()) };
    assert_eq!(at(&shifted, 0.0), 20.0);
    assert_eq!(at(&shifted, 2.0), 10.0);
    let past_one = Modulated { phase: Box::new(1.25.into()), ..modulated(rising()) };
    assert_eq!(at(&past_one, 0.0), 12.5);
  }

  #[test]
  fn a_phase_rising_through_the_scene_adds_waves() {
    let keys = vec![Key::new(0.0, 0.0, Curve::Linear), Key::new(4.0, 1.0, Curve::Linear)];
    let phase = Prop::Keyframes(Keyframes { keys, cycle: Cycle::Once });
    let faster = Modulated { phase: Box::new(phase), ..modulated(rising()) };
    // a wave from the rate and one from the phase, so two over the scene
    assert_eq!([0.0, 1.0, 2.0, 3.0].map(|time| at(&faster, time)), [10.0, 15.0, 10.0, 15.0]);
  }

  #[test]
  fn a_phase_can_be_a_param() {
    let params = Params::from([("offset".into(), Value::Float(0.5))]);
    let shifted = Modulated { phase: Box::new(Prop::Param("offset".into())), ..modulated(up_and_down()) };
    assert_eq!(shifted.resolve::<f32>(&env(0.0, &params)), 20.0);
  }

  #[test]
  fn every_part_of_a_vec2_and_a_color_moves_together() {
    let params = Params::new();
    let timing = Timing::new(4.0, Cycle::Forever);
    let env = Env { timing: Some(timing), ..env(2.0, &params) };
    let spread = Spread::Vec2 { x: (0.0, 10.0), y: (5.0, 5.0) };
    let point: Vec2 = Modulated { spread, ..modulated(rising()) }.resolve(&env);
    assert_eq!(point, Vec2::new(5.0, 5.0));

    let spread = Spread::Color { hue: (0.0, 240.0), brightness: (1.0, 1.0), alpha: (0.0, 1.0) };
    let color: Color = Modulated { spread, ..modulated(rising()) }.resolve(&env);
    assert_eq!(color.0, Rgba([0, 255, 0, 128]));
  }

  #[test]
  fn with_no_timing_it_holds_its_starting_value() {
    let params = Params::new();
    let shifted = Modulated { phase: Box::new(0.5.into()), ..modulated(up_and_down()) };
    assert_eq!(shifted.resolve::<f32>(&env(3.0, &params)), 20.0);
  }

  #[test]
  fn a_spread_that_doesnt_match_the_type_is_the_default() {
    let params = Params::new();
    assert_eq!(modulated(rising()).resolve::<Color>(&env(0.0, &params)), Color::default());
  }

  #[test]
  fn a_prop_reads_its_wave_and_is_animated() {
    let params = Params::new();
    let prop: Prop<f32> = Prop::Modulated(modulated(falling()));
    assert!(prop.is_animated());
    assert_eq!(prop.resolve(&env(0.0, &params)), 20.0);
  }
}
