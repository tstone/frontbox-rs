use std::f32::consts::PI;

/// Overshoot amount for the Back curves
const BACK_C1: f32 = 1.70158;
const BACK_C2: f32 = BACK_C1 * 1.525;
const BACK_C3: f32 = BACK_C1 + 1.0;

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Curve {
  #[default]
  Linear,
  EaseIn,
  EaseOut,
  EaseInOut,
  /// Eases in and out along half a cosine, so keys that alternate between two values make a sine wave
  SineInOut,
  ExponentialIn,
  ExponentialOut,
  ExponentialInOut,
  /// Rises from 0 to 1 and falls back to 0 over a single cycle
  Sinusoid,
  /// Pulls back below 0 before moving forward
  BackIn,
  /// Overshoots past 1 before settling
  BackOut,
  BackInOut,
  BounceIn,
  BounceOut,
  BounceInOut,
  /// Discrete steps instead of a continuous curve, with the given number of steps. Compose another curve into it to
  /// quantize that curve instead, e.g. `Curve::EaseIn.compose(Curve::Steps(4))`
  Steps(usize),
  Constant(f32),
  /// Smooth random wobble between 0 and 1 through the given number of random heights, from the given seed. The same
  /// seed always makes the same wobble, and the end leads back into the start, so it repeats without a jump. Shape it
  /// into a flame's flicker with `Reflect(Compose(Noise(16, seed), EaseIn))`: mostly high, with short dips.
  Noise(usize, u32),
  /// Runs the curve backwards in time, `c(1 - phase)`, so a curve that rises from 0 to 1 falls from 1 to 0
  Reverse(Box<Self>),
  /// Runs the curve backwards in time and flips it, `1 - c(1 - phase)`, turning an ease-in into an ease-out
  Reflect(Box<Self>),
  /// Plays the curve the given number of times over the phase, each time from its start to its end
  Repeat(Box<Self>, usize),
  /// Splits the phase into the given number of slots and freezes the curve through about half of them, picked at
  /// random from the given seed. A frozen slot holds where the curve was at its start, then the next slot snaps to
  /// where the curve has got to, like video that hangs and catches up.
  Hold(Box<Self>, usize, u32),
  /// Adds the outputs of two curves, e.g. `Add(Linear, Multiply(Sinusoid, Constant(0.2)))` for a rise with a bump on it
  Add(Box<Self>, Box<Self>),
  /// Multiplies the outputs of two curves, e.g. `Multiply(Sinusoid, EaseIn)` for a pulse that grows in intensity
  Multiply(Box<Self>, Box<Self>),
  /// Feeds the output of the first curve into the second, `second(first(phase))`
  Compose(Box<Self>, Box<Self>),
}

impl Curve {
  pub fn sample(&self, phase: f32) -> f32 {
    match self {
      Self::Linear => phase,
      Self::Constant(c) => *c,
      Self::EaseIn => phase.powi(2),
      Self::EaseOut => 1.0 - (1.0 - phase).powi(2),
      Self::EaseInOut => sample_ease_inout(phase),
      Self::SineInOut => (1.0 - f32::cos(phase * PI)) / 2.0,
      Self::ExponentialIn => 2.0f32.powf(10.0 * phase - 10.0),
      Self::ExponentialOut => 1.0 - 2.0f32.powf(-10.0 * phase),
      Self::ExponentialInOut => sample_exponential_inout(phase),
      Self::Sinusoid => sample_sinusoid(phase),
      Self::BounceIn => sample_bounce_in(phase),
      Self::BounceOut => sample_bounce_out(phase),
      Self::BounceInOut => sample_bounce_inout(phase),
      Self::BackIn => BACK_C3 * phase.powi(3) - BACK_C1 * phase.powi(2),
      Self::BackOut => 1.0 + BACK_C3 * (phase - 1.0).powi(3) + BACK_C1 * (phase - 1.0).powi(2),
      Self::BackInOut => sample_back_inout(phase),
      Self::Steps(steps) => sample_steps(steps.saturating_sub(1).clamp(1, usize::MAX), phase),
      Self::Noise(points, seed) => sample_noise(*points, *seed, phase),
      Self::Reverse(other) => other.sample(1.0 - phase),
      Self::Reflect(other) => 1.0 - other.sample(1.0 - phase),
      Self::Repeat(other, times) => other.sample(sample_repeat(*times, phase)),
      Self::Hold(other, slots, seed) => other.sample(sample_hold(*slots, *seed, phase)),
      Self::Add(a, b) => a.sample(phase) + b.sample(phase),
      Self::Multiply(a, b) => a.sample(phase) * b.sample(phase),
      Self::Compose(first, second) => second.sample(first.sample(phase)),
    }
  }

  /// Wraps the curve in [`Curve::Reverse`], unwrapping it instead if it's already reversed
  pub fn reverse(self) -> Self {
    match self {
      Curve::Reverse(inner) => *inner,
      other => Curve::Reverse(Box::new(other)),
    }
  }

  /// Wraps the curve in [`Curve::Reflect`], unwrapping it instead if it's already reflected
  pub fn reflect(self) -> Self {
    match self {
      Curve::Linear => Curve::Linear,
      Curve::Reflect(inner) => *inner,
      other => Curve::Reflect(Box::new(other)),
    }
  }

  /// Plays this curve `times` times over the phase; see [`Curve::Repeat`]
  pub fn repeat(self, times: usize) -> Self {
    Curve::Repeat(Box::new(self), times)
  }

  /// Freezes this curve through random slots of the phase; see [`Curve::Hold`]
  pub fn hold(self, slots: usize, seed: u32) -> Self {
    Curve::Hold(Box::new(self), slots, seed)
  }

  /// Adds `other`'s output to this curve's; see [`Curve::Add`]
  pub fn add(self, other: Self) -> Self {
    Curve::Add(Box::new(self), Box::new(other))
  }

  /// Multiplies this curve's output by `other`'s; see [`Curve::Multiply`]
  pub fn multiply(self, other: Self) -> Self {
    Curve::Multiply(Box::new(self), Box::new(other))
  }

  /// Feeds this curve's output into `next`, `next(self(phase))`; see [`Curve::Compose`]
  pub fn compose(self, next: Self) -> Self {
    Curve::Compose(Box::new(self), Box::new(next))
  }
}

#[inline]
fn sample_sinusoid(phase: f32) -> f32 {
  1.0 - (f32::cos(phase * 2. * PI) + 1.0) / 2.0
}

/// Where in its current repeat `phase` is, 0 to 1. The end of each repeat is 1 rather than the 0 the next one starts
/// at, so a repeated curve ends where the curve does.
fn sample_repeat(times: usize, phase: f32) -> f32 {
  let through = phase * times.max(1) as f32;
  if through <= 0.0 {
    through
  } else {
    through - (through.ceil() - 1.0)
  }
}

/// Value noise: a random height at each of `points` even spaces along the phase, eased between with smoothstep.
/// Wraps around, so phase 1 is back at the first height.
fn sample_noise(points: usize, seed: u32, phase: f32) -> f32 {
  let points = points.max(1) as i64;
  let through = phase * points as f32;
  let index = through.floor();
  let t = through - index;
  let index = index as i64;
  let from = noise_height(index.rem_euclid(points) as u32, seed);
  let to = noise_height((index + 1).rem_euclid(points) as u32, seed);
  from + (to - from) * t * t * (3.0 - 2.0 * t)
}

/// The phase to sample at: the start of its slot if the slot is frozen, or the phase itself. The slots repeat past the
/// end the same way, as the phase runs on into the next wave.
fn sample_hold(slots: usize, seed: u32, phase: f32) -> f32 {
  let slots = slots.max(1) as i64;
  let slot = (phase * slots as f32).floor();
  let frozen = noise_height((slot as i64).rem_euclid(slots) as u32, seed) < 0.5;
  if frozen { slot / slots as f32 } else { phase }
}

/// A random height from 0 to 1 for point `index`. Integer hashing, so the editor can make the same heights.
fn noise_height(index: u32, seed: u32) -> f32 {
  let mut x = seed.wrapping_mul(0x9e37_79b9).wrapping_add(index);
  x ^= x >> 16;
  x = x.wrapping_mul(0x7feb_352d);
  x ^= x >> 15;
  x = x.wrapping_mul(0x846c_a68b);
  x ^= x >> 16;
  // the top 24 bits, which an f32 holds exactly
  (x >> 8) as f32 / 0xff_ffff as f32
}

#[inline]
fn sample_steps(steps: usize, phase: f32) -> f32 {
  (phase * steps as f32).round() / steps.max(1) as f32
}

fn sample_ease_inout(phase: f32) -> f32 {
  if phase < 0.5 {
    2.0 * phase.powi(2)
  } else {
    1.0 - (-2.0 * phase + 2.0).powi(2) / 2.0
  }
}

fn sample_exponential_inout(phase: f32) -> f32 {
  if phase < 0.5 {
    2.0f32.powf(20.0 * phase - 10.0) / 2.0
  } else {
    (2.0 - 2.0f32.powf(-20.0 * phase + 10.0)) / 2.0
  }
}

fn sample_back_inout(phase: f32) -> f32 {
  if phase < 0.5 {
    (2.0 * phase).powi(2) * ((BACK_C2 + 1.0) * 2.0 * phase - BACK_C2) / 2.0
  } else {
    ((2.0 * phase - 2.0).powi(2) * ((BACK_C2 + 1.0) * (2.0 * phase - 2.0) + BACK_C2) + 2.0) / 2.0
  }
}

fn sample_bounce_in(phase: f32) -> f32 {
  1.0 - sample_bounce_out(1.0 - phase)
}

fn sample_bounce_out(phase: f32) -> f32 {
  let n1 = 7.5625;
  let d1 = 2.75;

  if phase < 1.0 / d1 {
    n1 * phase * phase
  } else if phase < 2.0 / d1 {
    let phase = phase - 1.5 / d1;
    n1 * phase * phase + 0.75
  } else if phase < 2.5 / d1 {
    let phase = phase - 2.25 / d1;
    n1 * phase * phase + 0.9375
  } else {
    let phase = phase - 2.625 / d1;
    n1 * phase * phase + 0.984375
  }
}

fn sample_bounce_inout(phase: f32) -> f32 {
  if phase < 0.5 {
    (1.0 - sample_bounce_out(1.0 - 2.0 * phase)) / 2.0
  } else {
    (1.0 + sample_bounce_out(2.0 * phase - 1.0)) / 2.0
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn easing_curves_start_at_zero_and_end_at_one() {
    let curves = [
      Curve::Linear,
      Curve::EaseIn,
      Curve::EaseOut,
      Curve::EaseInOut,
      Curve::SineInOut,
      Curve::ExponentialIn,
      Curve::ExponentialOut,
      Curve::ExponentialInOut,
      Curve::BackIn,
      Curve::BackOut,
      Curve::BackInOut,
      Curve::BounceIn,
      Curve::BounceOut,
      Curve::BounceInOut,
      Curve::Steps(4),
    ];

    for curve in curves {
      assert!(
        curve.sample(0.0).abs() < 0.01,
        "{curve:?} at 0.0 = {}",
        curve.sample(0.0)
      );
      assert!(
        (curve.sample(1.0) - 1.0).abs() < 0.01,
        "{curve:?} at 1.0 = {}",
        curve.sample(1.0)
      );
    }
  }

  #[test]
  fn reflect_turns_ease_in_into_ease_out() {
    for i in 0..=10 {
      let phase = i as f32 / 10.0;
      let reflected = Curve::EaseIn.reflect().sample(phase);
      assert!(
        (reflected - Curve::EaseOut.sample(phase)).abs() < 0.001,
        "at {phase}"
      );
    }
  }

  #[test]
  fn reverse_runs_the_curve_from_one_to_zero() {
    let reversed = Curve::EaseIn.reverse();
    assert_eq!(reversed.sample(0.0), 1.0);
    assert_eq!(reversed.sample(0.75), 0.0625);
    assert_eq!(reversed.sample(1.0), 0.0);
    assert_eq!(reversed.reverse(), Curve::EaseIn);
  }

  #[test]
  fn repeat_plays_the_curve_each_time_from_start_to_end() {
    let twice = Curve::EaseIn.repeat(2);
    assert_eq!(twice.sample(0.0), 0.0);
    assert_eq!(twice.sample(0.25), 0.25);
    assert_eq!(twice.sample(0.5), 1.0);
    assert_eq!(twice.sample(0.75), 0.25);
    assert_eq!(twice.sample(1.0), 1.0);
  }

  #[test]
  fn add_adds_outputs() {
    // a rise with a bump on it, still from 0 to 1
    let curve = Curve::Linear.add(Curve::Sinusoid.multiply(Curve::Constant(0.5)));
    assert_eq!(curve.sample(0.0), 0.0);
    assert_eq!(curve.sample(0.5), 1.0);
    assert_eq!(curve.sample(1.0), 1.0);
  }

  #[test]
  fn nested_curves_round_trip_through_ron() {
    let curve = Curve::BackOut
      .compose(Curve::Steps(3))
      .multiply(Curve::Sinusoid.reverse().reflect())
      .add(Curve::Sinusoid.repeat(4));
    let written = ron::to_string(&curve).unwrap();
    assert_eq!(
      written,
      "Add(Multiply(Compose(BackOut,Steps(3)),Reflect(Reverse(Sinusoid))),Repeat(Sinusoid,4))"
    );
    assert_eq!(ron::from_str::<Curve>(&written).unwrap(), curve);
  }

  #[test]
  fn noise_stays_between_zero_and_one_and_wraps_around() {
    let noise = Curve::Noise(16, 7);
    for i in 0..=1000 {
      let height = noise.sample(i as f32 / 1000.0);
      assert!((0.0..=1.0).contains(&height), "at {i}: {height}");
    }
    assert_eq!(noise.sample(0.0), noise.sample(1.0));
    // runs on past the end as it would into the next wave
    assert!((noise.sample(1.25) - noise.sample(0.25)).abs() < 1e-5);
  }

  #[test]
  fn noise_is_the_same_for_a_seed_and_different_between_seeds() {
    assert_eq!(
      Curve::Noise(16, 7).sample(0.3),
      Curve::Noise(16, 7).sample(0.3)
    );
    assert_ne!(
      Curve::Noise(16, 7).sample(0.3),
      Curve::Noise(16, 8).sample(0.3)
    );
    // the editor hashes the same way, so these are the heights to check its copy against
    assert_eq!(noise_height(0, 0), 0.0);
    assert_eq!(noise_height(1, 0), 0.40834907);
    assert_eq!(noise_height(3, 42), 0.046998683);
  }

  #[test]
  fn hold_freezes_random_slots_and_catches_up_after() {
    let held = Curve::Linear.hold(8, 5);
    let frozen: Vec<bool> = (0..8u32).map(|slot| noise_height(slot, 5) < 0.5).collect();
    assert!(
      frozen.contains(&true) && frozen.contains(&false),
      "{frozen:?}"
    );
    for (slot, frozen) in frozen.into_iter().enumerate() {
      let start = slot as f32 / 8.0;
      let middle = start + 1.0 / 16.0;
      let expected = if frozen { start } else { middle };
      assert_eq!(held.sample(middle), expected, "slot {slot}");
      // every slot starts where the curve is
      assert_eq!(held.sample(start), start, "slot {slot}");
    }
    assert_eq!(held.sample(1.0), 1.0);
  }

  #[test]
  fn hold_round_trips_through_ron() {
    let curve = Curve::EaseIn.hold(8, 5);
    let written = ron::to_string(&curve).unwrap();
    assert_eq!(written, "Hold(EaseIn,8,5)");
    assert_eq!(ron::from_str::<Curve>(&written).unwrap(), curve);
  }

  #[test]
  fn noise_round_trips_through_ron() {
    let curve = Curve::Noise(16, 3).compose(Curve::EaseIn).reflect();
    let written = ron::to_string(&curve).unwrap();
    assert_eq!(written, "Reflect(Compose(Noise(16,3),EaseIn))");
    assert_eq!(ron::from_str::<Curve>(&written).unwrap(), curve);
  }

  #[test]
  fn back_curves_overshoot_by_about_ten_percent() {
    let min_in = (0..=100)
      .map(|i| Curve::BackIn.sample(i as f32 / 100.0))
      .fold(f32::MAX, f32::min);
    let max_out = (0..=100)
      .map(|i| Curve::BackOut.sample(i as f32 / 100.0))
      .fold(f32::MIN, f32::max);
    assert!((-0.11..-0.09).contains(&min_in), "BackIn min = {min_in}");
    assert!((1.09..1.11).contains(&max_out), "BackOut max = {max_out}");
  }

  #[test]
  fn multiply_multiplies_outputs() {
    let curve = Curve::Linear.multiply(Curve::Constant(0.5));
    assert_eq!(curve.sample(0.5), 0.25);
    assert_eq!(curve.sample(1.0), 0.5);
  }

  #[test]
  fn compose_feeds_first_into_second() {
    // EaseIn(0.5) = 0.25, then Steps(5) snaps 0.25 to the nearest quarter
    let curve = Curve::EaseIn.compose(Curve::Steps(5));
    assert_eq!(curve.sample(0.5), 0.25);
    assert_eq!(curve.sample(0.6), 0.25);
    assert_eq!(curve.sample(1.0), 1.0);

    // order matters: Steps first, then EaseIn
    let curve = Curve::Steps(5).compose(Curve::EaseIn);
    assert_eq!(curve.sample(0.5), 0.25);
    assert_eq!(curve.sample(0.6), 0.25);
    assert_eq!(curve.sample(0.7), 0.5625);
  }
}
