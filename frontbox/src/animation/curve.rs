use std::{f32::consts::PI, fmt::Debug};

/// Overshoot amount for the Back curves
const BACK_C1: f32 = 1.70158;
const BACK_C2: f32 = BACK_C1 * 1.525;
const BACK_C3: f32 = BACK_C1 + 1.0;

#[derive(Debug, Default, Clone)]
pub enum Curve {
  #[default]
  Linear,
  EaseIn,
  EaseOut,
  EaseInOut,
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
  /// A new random value on every sample, ignoring phase
  Random,
  /// Discrete steps instead of a continuous curve, with the given number of steps
  Steps(usize),
  Constant(f32),
  Reverse(Box<Self>),
  /// Combines two curves by multiplying their outputs together, allowing for complex interactions between curve shapes
  /// e.g. Remap(Random, EaseIn) for a flicker that grows in intensity
  Remap(Box<Self>, Box<Self>),
}

impl Curve {
  pub fn sample(&self, phase: f32) -> f32 {
    match self {
      Self::Linear => phase,
      Self::Constant(c) => *c,
      Self::EaseIn => phase.powi(2),
      Self::EaseOut => 1.0 - (1.0 - phase).powi(2),
      Self::EaseInOut => sample_ease_inout(phase),
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
      Self::Random => rand::random::<f32>(),
      // should steps be a quantization of an existing Curve?
      Self::Steps(steps) => sample_steps(steps.saturating_sub(1).clamp(1, usize::MAX), phase),
      Self::Reverse(other) => 1.0 - other.sample(phase),
      Self::Remap(a, b) => a.sample(phase) * b.sample(phase),
    }
  }

  pub fn reverse(self) -> Self {
    Curve::Reverse(Box::new(self))
  }

  pub fn remap(self, other: Self) -> Self {
    Curve::Remap(Box::new(self), Box::new(other))
  }
}

#[inline]
fn sample_sinusoid(phase: f32) -> f32 {
  1.0 - (f32::cos(phase * 2. * PI) + 1.0) / 2.0
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
      assert!(curve.sample(0.0).abs() < 0.01, "{curve:?} at 0.0 = {}", curve.sample(0.0));
      assert!((curve.sample(1.0) - 1.0).abs() < 0.01, "{curve:?} at 1.0 = {}", curve.sample(1.0));
    }
  }

  #[test]
  fn back_curves_overshoot_by_about_ten_percent() {
    let min_in = (0..=100).map(|i| Curve::BackIn.sample(i as f32 / 100.0)).fold(f32::MAX, f32::min);
    let max_out = (0..=100).map(|i| Curve::BackOut.sample(i as f32 / 100.0)).fold(f32::MIN, f32::max);
    assert!((-0.11..-0.09).contains(&min_in), "BackIn min = {min_in}");
    assert!((1.09..1.11).contains(&max_out), "BackOut max = {max_out}");
  }
}
