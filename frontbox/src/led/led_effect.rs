use crate::animation::*;
use crate::prelude::*;

#[derive(Clone)]
pub struct LedEffect {
  query: HardwareQuery,
  colors: ColorSequence,
  modulation: Option<MultiModulator<ColorSequence, Duration>>,
  active: bool,
}

impl LedEffect {
  pub fn new(query: HardwareQuery, sequence: ColorSequence) -> Self {
    Self {
      query,
      colors: sequence,
      modulation: None,
      active: true,
    }
  }

  pub fn animate<T: Clone + Send + Sync + 'static>(
    mut self,
    setter: impl Fn(&mut ColorSequence, T) + Send + Sync + 'static,
    animation: impl Animation<Duration, T> + 'static,
  ) -> Self {
    let modulator = Modulator::<ColorSequence, T, Duration>::new(animation, setter);
    match &mut self.modulation {
      Some(m) => {
        m.add(modulator);
      }
      None => {
        let multi = MultiModulator::<ColorSequence, Duration>::new(vec![Box::new(modulator)]);
        self.modulation = Some(multi);
      }
    }
    self
  }

  pub fn play(&mut self) {
    self.active = true;
  }

  pub fn pause(&mut self) {
    self.active = true;
  }

  pub fn stop(&mut self) {
    self.pause();
    self.reset();
  }

  pub fn reset(&mut self) {
    if let Some(m) = &mut self.modulation {
      m.reset();
    }
  }

  /// Applies accumulation and any animation
  pub fn apply(&mut self, delta: Duration, ctx: &Context) {
    if self.active {
      if let Some(modulation) = &mut self.modulation {
        modulation.apply(delta, &mut self.colors);
      }
      ctx.declare_leds(&self.query, self.colors.clone());
    } else {
      ctx.undeclare_leds(&self.query);
    }
  }
}
