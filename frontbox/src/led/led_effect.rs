use crate::animation::*;
use crate::prelude::color_sequence::Modification;
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

  /// Flash all LEDs on and off. Duration is a full on/off cycle
  pub fn flash(query: HardwareQuery, color: Rgba<u8>, duration: Duration) -> Self {
    Self::new(query, ColorSequence::tile(vec![color])).animate(
      |seq, value| *seq.fill.pattern_mut().unwrap() = vec![value],
      Tween::new(
        duration / 2,
        Curve::Steps(2),
        vec![color, Rgba::default()],
        Cycle::Forever,
      ),
    )
  }

  /// Rotate the given color sequence
  pub fn rotate(query: HardwareQuery, colors: ColorSequence, duration: Duration) -> Self {
    Self::new(query, colors.modify(Modification::rotated(0.0))).animate(
      |seq, value| *seq.modifications[0].rotation_mut().unwrap() = value,
      Tween::new(duration, Curve::Linear, vec![0.0f32, 360.0], Cycle::Forever),
    )
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
