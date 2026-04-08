use std::{f32::consts::PI, fmt::Debug};

use noise::{MultiFractal, NoiseFn};

#[derive(Debug, Default, Clone)]
pub enum Curve {
  #[default]
  Linear,
  QuadraticIn,
  QuadraticOut,
  QuadraticInOut,
  EaseIn,
  EaseOut,
  EaseInOut,
  ExponentialIn,
  ExponentialOut,
  ExponentialInOut,
  Sinusoid,
  ElasticIn,
  ElasticOut,
  ElasticInOut,
  BackIn,
  BackOut,
  BackInOut,
  BounceIn,
  BounceOut,
  BounceInOut,
  SineIn,
  SineOut,
  SineInOut,
  Random,
  SmoothRandom,
  /// Organic noise that is similar to Random but with smooth transitions between values, seeded by the given u32
  SimplexNoise(u32),
  /// Cracked/cellular noise that produces random flat regions separated by sharp edges, seeded by the given u32
  WorleyNoise(u32),
  /// Stormy, turbulent noise with a fractal structure, seeded by the given u32 and with the given number of octaves
  FractalBrownianMotion(u32, usize),
  /// Mostly linear but with random spikes of intensity that increase with the given intensity parameter (0.0 to 1.0)
  Glitch(f32),
  /// Discrete steps instead of a continuous curve, with the given number of steps
  Steps(usize),
  /// Steps but with with random dropouts that increase with the given intensity parameter (0.0 to 1.0)
  Stutter(usize, f32),
  Constant(f32),
  Reverse(Box<Self>),
  /// Combines two curves by multiplying their outputs together, allowing for complex interactions between curve shapes
  /// Most useful for combining a simple curve with noise, e.g. Remap(Glitch(0.1), EaseOut)
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
      Self::EaseIn => phase.powf(2.0),
      Self::EaseOut => 1.0 - (1.0 - phase).powf(2.0),
      Self::EaseInOut => sample_quadratic_inout(phase),
      Self::ExponentialIn => 2.0f32.powf(10.0 * phase - 10.0),
      Self::ExponentialOut => 1.0 - 2.0f32.powf(-10.0 * phase),
      Self::ExponentialInOut => sample_exponential_inout(phase),
      Self::Sinusoid => sample_sinusoid(phase),
      Self::BounceIn => sample_bounce_in(phase),
      Self::BounceOut => sample_bounce_out(phase),
      Self::BounceInOut => sample_bounce_inout(phase),
      Self::ElasticIn => sample_elastic_in(phase),
      Self::ElasticOut => sample_elastic_out(phase),
      Self::ElasticInOut => sample_elastic_inout(phase),
      Self::BackIn => phase.powf(2.0) * ((phase * 2.0 - 1.0) * 2.7 + 1.7),
      Self::BackOut => 1.0 - (1.0 - phase).powf(2.0) * (((1.0 - phase) * 2.0 - 1.0) * 2.7 + 1.7),
      Self::BackInOut => {
        if phase < 0.5 {
          (phase * 2.0).powf(2.0) * ((phase * 4.0 - 1.0) * 2.7 + 1.7) / 2.0
        } else {
          1.0 - (1.0 - phase * 2.0).powf(2.0) * (((1.0 - phase) * 4.0 - 1.0) * 2.7 + 1.7) / 2.0
        }
      }
      Self::SineIn => 1.0 - (phase * PI / 2.0).cos(),
      Self::SineOut => (phase * PI / 2.0).sin(),
      Self::SineInOut => -(f32::cos(PI * phase) - 1.0) / 2.0,
      Self::Random => rand::random::<f32>(),
      Self::SmoothRandom => {
        let x = (phase * 1000.0) as u32;
        let x = x ^ (x << 13) ^ (x >> 17);
        x as f32 / u32::MAX as f32
      }
      Self::SimplexNoise(seed) => noise::Simplex::new(*seed).get([phase as f64, 0.0]) as f32,
      Self::WorleyNoise(seed) => noise::Worley::new(*seed).get([phase as f64, 0.0]) as f32,
      Self::FractalBrownianMotion(seed, octaves) => noise::Fbm::<noise::Perlin>::new(*seed)
        .set_octaves(*octaves)
        .get([phase as f64, 0.0]) as f32,
      Self::Glitch(intensity) => {
        let x = (phase * 1000.0) as u32;
        let hash = x ^ (x << 13) ^ (x >> 17);
        let spike = (hash as f32 / u32::MAX as f32) < *intensity;
        if spike { 1.0 - phase } else { phase }
      }
      Self::Stutter(steps, dropout) => {
        let stepped = (phase * *steps as f32).floor() / *steps as f32;
        let x = (phase * 1000.0) as u32;
        let hash = x ^ (x << 13);
        let dead = (hash as f32 / u32::MAX as f32) < *dropout;
        if dead { 0.0 } else { stepped }
      }

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

fn sample_elastic_in(phase: f32) -> f32 {
  if phase == 0.0 {
    0.0
  } else if phase == 1.0 {
    1.0
  } else {
    -2.0f32.powf(10.0 * phase - 10.0) * (phase * 10.0 - 10.75).sin()
  }
}

fn sample_elastic_out(phase: f32) -> f32 {
  if phase == 0.0 {
    0.0
  } else if phase == 1.0 {
    1.0
  } else {
    2.0f32.powf(-10.0 * phase) * (phase * 10.0 - 0.75).sin() + 1.0
  }
}

fn sample_elastic_inout(phase: f32) -> f32 {
  if phase == 0.0 {
    0.0
  } else if phase == 1.0 {
    1.0
  } else if phase < 0.5 {
    -(2.0f32.powf(20.0 * phase - 10.0) * (20.0 * phase - 11.125).sin()) / 2.0
  } else {
    (2.0f32.powf(-20.0 * phase + 10.0) * (20.0 * phase - 11.125).sin()) / 2.0 + 1.0
  }
}
