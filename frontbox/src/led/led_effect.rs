use crate::animation::*;
use crate::prelude::color_sequence::Modification;
use crate::prelude::*;

#[derive(Clone)]
pub struct LedEffect {
  query: HardwareQuery,
  colors: ColorSequence,
  modulation: Option<MultiModulator<ColorSequence, Duration>>,
  active: bool,
  pending_deactivation: bool,
}

impl LedEffect {
  pub fn new(query: HardwareQuery, sequence: ColorSequence) -> Self {
    Self {
      query,
      colors: sequence,
      modulation: None,
      active: true,
      pending_deactivation: false,
    }
  }

  /// Apply a modulation to the ColorSequence
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

  /// Cycling through the multiple ColorSequences, including the base one
  /// `duration` - Time each ColorSequence stays before moving to the next
  /// For abrupt changes use Curve::Steps(N), where N is the total ColorSequences in the cycle
  pub fn cycle(self, others: Vec<ColorSequence>, duration: Duration, curve: Curve) -> Self {
    let mut seq_of_seq = others.clone();
    seq_of_seq.insert(0, self.colors.clone());
    self.animate(
      |seq, cs| *seq = cs,
      Tween::new(
        duration * (others.len() as u32 + 1),
        // Curve::Steps(seq_of_seq.len()),
        curve,
        seq_of_seq,
        Cycle::Forever,
      ),
    )
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
  pub fn rotate(
    query: HardwareQuery,
    colors: ColorSequence,
    duration: Duration,
    direction: RotationDirection,
  ) -> Self {
    let stops = match direction {
      RotationDirection::Clockwise => vec![0.0f32, 360.0],
      RotationDirection::CounterClockwise => vec![360.0f32, 0.0],
    };
    Self::new(query, colors.modify(Modification::rotated(0.0))).animate(
      // TODO: the real fix is to attach modifications to the effect not the sequence
      |seq, value| {
        if let Some(modification) = seq.modifications.get_mut(0) {
          *modification.rotation_mut().unwrap() = value;
        } else {
          seq.modifications.push(Modification::rotated(value));
        }
      },
      Tween::new(duration, Curve::Linear, stops, Cycle::Forever),
    )
  }

  /// Sets the effect as active
  pub fn play(&mut self) {
    self.active = true;
  }

  /// Stops the effect as active but does not clear state or LEDs
  pub fn pause(&mut self) {
    self.active = false;
  }

  /// Stops the effect and clears current state
  pub fn stop(&mut self) {
    self.pause();
    self.reset();
  }

  pub fn reset(&mut self) {
    if let Some(m) = &mut self.modulation {
      m.reset();
    }
    log::debug!("pending_deactivation = true");
    self.pending_deactivation = true;
  }

  pub fn clear(&mut self, ctx: &Context) {
    ctx.undeclare_leds(&self.query);
    self.pending_deactivation = false;
  }

  /// Applies accumulation and any animation
  pub fn apply(&mut self, delta: Duration, ctx: &Context) {
    if self.active {
      if let Some(modulation) = &mut self.modulation {
        modulation.apply(delta, &mut self.colors);
      }
      ctx.declare_leds(&self.query, self.colors.clone());
    } else if self.pending_deactivation {
      self.clear(ctx);
    }
  }
}

pub enum RotationDirection {
  Clockwise,
  CounterClockwise,
}
