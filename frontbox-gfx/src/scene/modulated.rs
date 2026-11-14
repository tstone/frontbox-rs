use std::f32::consts::TAU;

use serde::{Deserialize, Serialize};

use crate::scene::{Animatable, Env, Prop, Spread};

/// A value that moves through its spread in a repeating wave, timed to the scene cycle
///
/// A [`Track`](crate::scene::Track)'s keys are in seconds, so a wave made of keys drifts when the scene's duration
/// changes. This keeps the same number of waves per cycle at any duration.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Modulated {
  pub wave: Wave,
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
    let height = self.wave.sample(self.position(env));
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

/// The shape of one wave of a [`Modulated`]. Each is lowest at the start of the wave, except `Saw`, which falls from its
/// highest. `Sine`, `Triangle`, and `Square` peak halfway through, so at the same rate they line up.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Wave {
  Sine,
  Triangle,
  /// High for `duty` of each wave (0 to 1), centered halfway through, and low for the rest
  Square { duty: f32 },
  /// Rises steadily, then jumps back to low
  Ramp,
  /// Jumps to high, then falls steadily
  Saw,
}

impl Wave {
  /// Its height, 0 to 1, at `position` through the wave, 0 to 1
  fn sample(self, position: f32) -> f32 {
    match self {
      Self::Sine => (1.0 - (position * TAU).cos()) / 2.0,
      Self::Triangle => 1.0 - (2.0 * position - 1.0).abs(),
      Self::Square { duty } => {
        let half = duty.clamp(0.0, 1.0) / 2.0;
        (0.5 - half..0.5 + half).contains(&position) as u8 as f32
      }
      Self::Ramp => position,
      Self::Saw => 1.0 - position,
    }
  }
}

#[cfg(test)]
mod tests {
  use glam::Vec2;
  use image::Rgba;

  use super::*;
  use crate::animation::Cycle;
  use crate::scene::value::tests::env;
  use crate::animation::Curve;
  use crate::scene::{Color, Key, Params, Timing, Track, Value};

  fn modulated(wave: Wave) -> Modulated {
    Modulated { wave, spread: Spread::Float(10.0, 20.0), rate: 1.0, phase: Box::new(0.0.into()) }
  }

  /// Its value at a moment of a scene 4 seconds long that loops forever
  fn at(modulated: &Modulated, scene_time: f32) -> f32 {
    let params = Params::new();
    let timing = Timing::new(4.0, Cycle::Forever);
    let env = Env { cycle: timing.cycle_index(scene_time), timing: Some(timing), ..env(timing.local_time(scene_time), &params) };
    modulated.resolve(&env)
  }

  fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
  }

  #[test]
  fn each_wave_has_its_shape() {
    let samples = |wave| [0.0, 1.0, 2.0, 3.0].map(|time| at(&modulated(wave), time));
    assert_eq!(samples(Wave::Triangle), [10.0, 15.0, 20.0, 15.0]);
    assert_eq!(samples(Wave::Ramp), [10.0, 12.5, 15.0, 17.5]);
    assert_eq!(samples(Wave::Saw), [20.0, 17.5, 15.0, 12.5]);
    assert_eq!(samples(Wave::Square { duty: 0.5 }), [10.0, 20.0, 20.0, 10.0]);
    let sine = samples(Wave::Sine);
    assert!(close(sine[0], 10.0) && close(sine[1], 15.0) && close(sine[2], 20.0) && close(sine[3], 15.0), "{sine:?}");
  }

  #[test]
  fn a_square_is_high_for_its_duty_around_the_middle() {
    let quarter = modulated(Wave::Square { duty: 0.25 });
    assert_eq!(at(&quarter, 1.4), 10.0);
    assert_eq!(at(&quarter, 1.6), 20.0);
    assert_eq!(at(&quarter, 2.4), 20.0);
    assert_eq!(at(&quarter, 2.6), 10.0);
    for time in [0.0, 2.0, 3.99] {
      assert_eq!(at(&modulated(Wave::Square { duty: 0.0 }), time), 10.0);
      assert_eq!(at(&modulated(Wave::Square { duty: 1.0 }), time), 20.0);
    }
  }

  #[test]
  fn rate_is_waves_per_cycle() {
    let twice = Modulated { rate: 2.0, ..modulated(Wave::Triangle) };
    assert_eq!([0.0, 1.0, 2.0, 3.0].map(|time| at(&twice, time)), [10.0, 20.0, 10.0, 20.0]);
  }

  #[test]
  fn a_fractional_rate_carries_on_into_the_next_cycle() {
    let half = Modulated { rate: 0.5, ..modulated(Wave::Ramp) };
    assert_eq!(at(&half, 0.0), 10.0);
    assert_eq!(at(&half, 4.0), 15.0);
    assert_eq!(at(&half, 6.0), 17.5);
    assert_eq!(at(&half, 8.0), 10.0);
  }

  #[test]
  fn phase_moves_where_the_scene_starts() {
    let shifted = Modulated { phase: Box::new(0.5.into()), ..modulated(Wave::Triangle) };
    assert_eq!(at(&shifted, 0.0), 20.0);
    assert_eq!(at(&shifted, 2.0), 10.0);
    let past_one = Modulated { phase: Box::new(1.25.into()), ..modulated(Wave::Ramp) };
    assert_eq!(at(&past_one, 0.0), 12.5);
  }

  #[test]
  fn a_phase_rising_through_the_scene_adds_waves() {
    let keys = vec![Key::new(0.0, 0.0, Curve::Linear), Key::new(4.0, 1.0, Curve::Linear)];
    let phase = Prop::Track(Track { keys, cycle: Cycle::Once });
    let faster = Modulated { phase: Box::new(phase), ..modulated(Wave::Ramp) };
    // a wave from the rate and one from the phase, so two over the scene
    assert_eq!([0.0, 1.0, 2.0, 3.0].map(|time| at(&faster, time)), [10.0, 15.0, 10.0, 15.0]);
  }

  #[test]
  fn a_phase_can_be_a_param() {
    let params = Params::from([("offset".into(), Value::Float(0.5))]);
    let shifted = Modulated { phase: Box::new(Prop::Param("offset".into())), ..modulated(Wave::Triangle) };
    assert_eq!(shifted.resolve::<f32>(&env(0.0, &params)), 20.0);
  }

  #[test]
  fn every_part_of_a_vec2_and_a_color_moves_together() {
    let params = Params::new();
    let timing = Timing::new(4.0, Cycle::Forever);
    let env = Env { timing: Some(timing), ..env(2.0, &params) };
    let spread = Spread::Vec2 { x: (0.0, 10.0), y: (5.0, 5.0) };
    let point: Vec2 = Modulated { spread, ..modulated(Wave::Ramp) }.resolve(&env);
    assert_eq!(point, Vec2::new(5.0, 5.0));

    let spread = Spread::Color { hue: (0.0, 240.0), brightness: (1.0, 1.0), alpha: (0.0, 1.0) };
    let color: Color = Modulated { spread, ..modulated(Wave::Ramp) }.resolve(&env);
    assert_eq!(color.0, Rgba([0, 255, 0, 128]));
  }

  #[test]
  fn with_no_timing_it_holds_its_starting_value() {
    let params = Params::new();
    let shifted = Modulated { phase: Box::new(0.5.into()), ..modulated(Wave::Triangle) };
    assert_eq!(shifted.resolve::<f32>(&env(3.0, &params)), 20.0);
  }

  #[test]
  fn a_spread_that_doesnt_match_the_type_is_the_default() {
    let params = Params::new();
    assert_eq!(modulated(Wave::Sine).resolve::<Color>(&env(0.0, &params)), Color::default());
  }

  #[test]
  fn a_prop_reads_its_wave_and_is_animated() {
    let params = Params::new();
    let prop: Prop<f32> = Prop::Modulated(modulated(Wave::Saw));
    assert!(prop.is_animated());
    assert_eq!(prop.resolve(&env(0.0, &params)), 20.0);
  }
}
