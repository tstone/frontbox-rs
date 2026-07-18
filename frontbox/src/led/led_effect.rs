use crate::animation::*;
use crate::prelude::*;

pub struct LedEffect<S: ColorSequence + Clone + 'static> {
  query: HardwareQuery,
  colors: S,
  modulation: Option<MultiModulator<S, Duration>>,
}

impl<S> LedEffect<S>
where
  S: ColorSequence + Clone,
{
  pub fn new(query: HardwareQuery, starting_colors: S) -> Self {
    Self {
      query,
      colors: starting_colors,
      modulation: None,
    }
  }

  pub fn animate<T: Clone + Send + Sync + 'static>(
    mut self,
    setter: impl Fn(&mut S, T) + Send + Sync + 'static,
    animation: impl Animation<Duration, T> + 'static,
  ) -> Self {
    let modulator = Modulator::<S, T, Duration>::new(animation, setter);
    match &mut self.modulation {
      Some(m) => {
        m.add(modulator);
      }
      None => {
        let multi = MultiModulator::<S, Duration>::new(vec![Box::new(modulator)]);
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
