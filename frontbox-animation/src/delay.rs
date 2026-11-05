use crate::*;

/// Waits a set amount before starting another animation. While waiting it samples the wrapped animation's starting
/// value. Useful for staggering animations that run side by side, e.g. in a [`MultiModulator`].
///
/// ```rust
/// # use std::time::Duration;
/// # use frontbox_animation::*;
/// let mut anim = Delay::new(
///   Duration::from_secs(1),
///   Tween::new(Duration::from_secs(1), Curve::Linear, vec![0, 100], Cycle::Once),
/// );
///
/// anim.accumulate(Duration::from_millis(500));
/// assert_eq!(anim.sample(), 0);
///
/// anim.accumulate(Duration::from_secs(1));
/// assert_eq!(anim.sample(), 50);
/// ```
#[derive(Clone)]
pub struct Delay<A, T> {
  /// How long to wait before the animation starts
  pub delay: A,
  animation: Box<dyn Animation<A, T>>,
  elapsed: A,
  active: bool,
}

impl<A, T> Delay<A, T>
where
  A: Tweenable,
{
  /// Plays `animation` once `delay` has passed. The animation is set playing; use `.stopped()` on the delay to wait
  /// for `play()`.
  pub fn new(delay: A, animation: impl Animation<A, T> + 'static) -> Self {
    let mut animation: Box<dyn Animation<A, T>> = Box::new(animation);
    animation.play();

    Self {
      delay,
      animation,
      elapsed: A::default(),
      active: true,
    }
  }

  pub fn boxed(delay: A, animation: impl Animation<A, T> + 'static) -> Box<Self> {
    Box::new(Self::new(delay, animation))
  }

  /// Delay starts in a playing state by default. Use this to chain it to prevent that.
  pub fn stopped(mut self) -> Self {
    self.active = false;
    self.animation.stop();
    self
  }

  /// The amount left before the animation starts
  pub fn remaining(&self) -> A {
    if self.elapsed >= self.delay {
      A::default()
    } else {
      self.delay - self.elapsed
    }
  }
}

impl<A, T> Accumulator<A> for Delay<A, T>
where
  A: Tweenable,
  T: Clone,
{
  fn accumulate(&mut self, delta: A) -> AccumulationResult<A> {
    if !self.active || self.is_complete() {
      return AccumulationResult::default();
    }

    let mut delta = delta;
    if self.elapsed < self.delay {
      self.elapsed = self.elapsed + delta;
      if self.elapsed < self.delay {
        return AccumulationResult::default();
      }

      // whatever is left over after the delay starts the animation
      delta = self.elapsed - self.delay;
      self.elapsed = self.delay;
    }

    self.animation.accumulate(delta)
  }

  fn force(&mut self, current: A) {
    self.animation.reset();
    if current < self.delay {
      self.elapsed = current;
    } else {
      self.elapsed = self.delay;
      self.animation.force(current - self.delay);
    }
  }

  fn reset(&mut self) {
    self.elapsed = A::default();
    self.animation.reset();
  }

  fn is_complete(&self) -> bool {
    self.elapsed >= self.delay && self.animation.is_complete()
  }
}

impl<A, T> Animation<A, T> for Delay<A, T>
where
  A: Tweenable + 'static,
  T: Clone + Send + Sync + 'static,
{
  fn sample(&self) -> T {
    self.animation.sample()
  }

  fn play(&mut self) {
    self.active = true;
    self.animation.play();
  }

  fn stop(&mut self) {
    self.active = false;
    self.animation.stop();
  }

  fn active(&self) -> bool {
    self.active
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn delayed_tween() -> Delay<f32, f32> {
    Delay::new(1.0, Tween::new(1.0, Curve::Linear, vec![0.0, 10.0], Cycle::Once))
  }

  #[test]
  fn test_samples_start_while_waiting() {
    let mut delay = delayed_tween();
    delay.accumulate(0.5);
    assert_eq!(delay.sample(), 0.0);
    assert_eq!(delay.remaining(), 0.5);
    assert!(!delay.is_complete());
  }

  #[test]
  fn test_leftover_time_starts_the_animation() {
    let mut delay = delayed_tween();
    delay.accumulate(1.25);
    assert_eq!(delay.sample(), 2.5);
    assert_eq!(delay.remaining(), 0.0);
  }

  #[test]
  fn test_completes_with_the_animation() {
    let mut delay = delayed_tween();
    let result = delay.accumulate(2.5);
    assert!(result.completed_cycle);
    assert_eq!(result.remainder, 0.5);
    assert!(delay.is_complete());
    assert_eq!(delay.sample(), 10.0);
  }

  #[test]
  fn test_zero_delay_plays_immediately() {
    let mut delay = Delay::new(0.0f32, Tween::new(1.0, Curve::Linear, vec![0.0f32, 10.0], Cycle::Once));
    delay.accumulate(0.5);
    assert_eq!(delay.sample(), 5.0);
  }

  #[test]
  fn test_stopped_waits_for_play() {
    let mut delay = delayed_tween().stopped();
    delay.accumulate(1.5);
    assert_eq!(delay.remaining(), 1.0);

    delay.play();
    delay.accumulate(1.5);
    assert_eq!(delay.sample(), 5.0);
  }

  #[test]
  fn test_stop_pauses_the_animation() {
    let mut delay = delayed_tween();
    delay.accumulate(1.5);
    delay.stop();
    delay.accumulate(0.25);
    assert_eq!(delay.sample(), 5.0);
  }

  #[test]
  fn test_force_and_reset() {
    let mut delay = delayed_tween();

    delay.force(1.5);
    assert_eq!(delay.sample(), 5.0);

    delay.force(0.5);
    assert_eq!(delay.sample(), 0.0);
    assert_eq!(delay.remaining(), 0.5);

    delay.accumulate(2.0);
    delay.reset();
    assert_eq!(delay.sample(), 0.0);
    assert_eq!(delay.remaining(), 1.0);
  }

  #[test]
  fn test_inside_a_sequence() {
    // a delay in a sequence plays again on each cycle
    let mut seq: Sequence<f32, f32> = Sequence::new(vec![Box::new(delayed_tween())], Cycle::Forever);

    seq.accumulate(2.5);
    assert_eq!(seq.sample(), 0.0);

    seq.accumulate(1.0);
    assert_eq!(seq.sample(), 5.0);
  }
}
