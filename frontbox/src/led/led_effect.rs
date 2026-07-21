use crate::animation::*;
use crate::prelude::*;

#[derive(Clone)]
pub struct LedEffect {
  query: HardwareQuery,
  colors: ColorSequence,
  modulation: Option<MultiModulator<ColorSequence, Duration>>,
}

impl LedEffect {
  pub fn new(query: HardwareQuery, sequence: ColorSequence) -> Self {
    Self {
      query,
      colors: sequence,
      modulation: None,
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

  /// Applies accumulation and any animation
  pub fn apply(&mut self, delta: Duration, ctx: &Context) {
    if let Some(modulation) = &mut self.modulation {
      modulation.apply(delta, &mut self.colors);
    }
    ctx.declare_leds(&self.query, self.colors.clone());
  }
}
