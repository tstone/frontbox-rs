use std::{f32::consts::PI, fmt::Debug};

#[derive(Debug, Default, Clone)]
pub enum Curve {
  #[default]
  Linear,
  QuadraticIn,
  QuadraticOut,
  QuadraticInOut,
  ExponentialIn,
  ExponentialOut,
  ExponentialInOut,
  Sinusoid,
  Constant(f32),
  Steps(usize),
  Reverse(Box<Self>),
  Remap(Box<Self>, Box<Self>),
}

impl Curve {
  pub fn sample(&self, phase: f32) -> f32 {
    match self {
      Self::Linear => phase,
      Self::Constant(c) => *c,
      Self::QuadraticIn => phase.powi(2),
      Self::QuadraticOut => 1.0 - (1.0 - phase).powi(2),
      Self::QuadraticInOut => sample_quadratic_inout(phase),
      Self::ExponentialIn => 2.0f32.powf(10.0 * phase - 10.0),
      Self::ExponentialOut => 1.0 - 2.0f32.powf(-10.0 * phase),
      Self::ExponentialInOut => sample_exponential_inout(phase),
      Self::Sinusoid => sample_sinusoid(phase),
      Self::Steps(steps) => sample_steps(*steps, phase), // should steps be a quantization of an existing Curve?
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

fn sample_quadratic_inout(phase: f32) -> f32 {
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
