use crate::animation::*;
use crate::prelude::color_sequence::ColorSequenceAlteration;
use crate::prelude::*;

#[derive(Clone)]
pub struct LedEffect {
  query: HardwareQuery,
  anim: Box<dyn Animation<Duration, ColorSequence>>,
  alterations: Vec<LedEffectAlteration>,
  active: bool,
}

impl LedEffect {
  pub fn new(
    query: HardwareQuery,
    anim: impl Animation<Duration, ColorSequence> + 'static,
  ) -> Self {
    Self {
      query,
      anim: Box::new(anim), // Tween::new(duration, curve, sequences, cycle),
      alterations: Vec::new(),
      active: true,
    }
  }

  /// Cycle (tween) through all given ColorSequences, over `duration`.
  /// For abrupt changes use Curve::Steps(N), where N is the total ColorSequences in the cycle
  ///
  /// ```rust,ignore
  /// // fade everything from red to blue
  /// LedEffect::cycle(q, Duration::from_secs(1), Curve::Linear, vec![
  ///   ColorSequence::solid(Rgba::blue()),
  ///   ColorSequence::solid(Rgba::red()),
  /// ])
  ///
  /// // fade between all red to striped red
  /// LedEffect::cycle(q, Duration::from_secs(1), Curve::Linear, vec![
  ///   ColorSequence::solid(Rgba::red()),
  ///   ColorSequence::tile(vec![Rgba::red(), Rgba::white()]),
  /// ])
  ///
  /// // "dancing lights" effect
  /// LedEffect::cycle(q, Duration::from_secs(1), Curve::Steps(2), vec![
  ///   ColorSequence::tile(vec![Rgba::white(), Rgba::red()]),
  ///   ColorSequence::tile(vec![Rgba::red(), Rgba::white()]),
  /// ])
  /// ```
  pub fn cycle(
    query: HardwareQuery,
    duration: Duration,
    curve: Curve,
    sequences: Vec<ColorSequence>,
  ) -> Self {
    Self {
      query,
      anim: Box::new(Tween::new(duration, curve, sequences, Cycle::Forever)),
      alterations: Vec::new(),
      active: true,
    }
  }

  /// Flash all LEDs on and off. Duration is a full on/off cycle
  pub fn flash(
    query: HardwareQuery,
    color1: Rgba<u8>,
    color2: Rgba<u8>,
    duration: Duration,
  ) -> Self {
    Self::cycle(
      query,
      duration,
      Curve::EaseInOut,
      vec![ColorSequence::solid(color1), ColorSequence::solid(color2)],
    )
  }

  pub fn flash_on_off(query: HardwareQuery, color: Rgba<u8>, duration: Duration) -> Self {
    Self::flash(query, color, Rgba::default(), duration)
  }

  pub fn rotating(mut self, duration: Duration, curve: Curve) -> Self {
    self
      .alterations
      .push(LedEffectAlteration::Rotating(Rotating::new(
        duration, curve,
      )));
    self
  }

  pub fn shuffled(mut self, seed: u64) -> Self {
    self.alterations.push(LedEffectAlteration::Static(
      ColorSequenceAlteration::Shuffle(seed),
    ));
    self
  }

  pub fn reset(&mut self) {
    for alteration in &mut self.alterations {
      alteration.reset();
    }
  }

  /// Remove LED effects from being applied
  pub fn clear(&mut self, ctx: &Context) {
    ctx.undeclare_leds(&self.query);
  }

  pub fn resume(&mut self) {
    self.active = true;
  }

  pub fn stop(&mut self) {
    self.active = false;
    self.reset();
  }

  /// Remove LED effects from being applied and stop applying them in the future
  pub fn stop_and_clear(&mut self, ctx: &Context) {
    self.stop();
    self.clear(ctx);
  }

  /// Applies accumulation and any animation
  pub fn apply(&mut self, delta: Duration, ctx: &Context) {
    if self.active {
      self.anim.accumulate(delta);

      let mut base = self.anim.sample();
      for alteration in &mut self.alterations {
        base.alter(alteration.apply(delta));
      }

      ctx.declare_leds(&self.query, base);
    }
  }
}
