use crate::animation::*;

/// Plays a sequence of animations in order
#[derive(Clone)]
pub struct Sequence<A, T> {
  sequence: Vec<Box<dyn Animation<A, T>>>,
  current_anim_index: usize,
  cycle: Cycle,
  cycle_count: u32,
  active: bool,
}

impl<A, T> Sequence<A, T>
where
  T: Clone,
  A: Copy + Default + PartialEq,
{
  pub fn new(sequence: Vec<Box<dyn Animation<A, T>>>, cycle: Cycle) -> Self {
    assert!(!sequence.is_empty(), "Sequence requires at least 1 animation");

    Self {
      sequence,
      current_anim_index: 0,
      cycle,
      cycle_count: 0,
      active: true,
    }
  }

  pub fn boxed(sequence: Vec<Box<dyn Animation<A, T>>>, cycle: Cycle) -> Box<Self> {
    Box::new(Self::new(sequence, cycle))
  }

  /// Sequence starts in a playing state by default. Use this to chain it to prevent that.
  pub fn stopped(mut self) -> Self {
    self.stop();
    self
  }

  /// Reset all child animations, leaving only the first playing
  fn reset_all(&mut self) {
    for anim in &mut self.sequence {
      anim.reset();
      anim.stop();
    }
    if let Some(first) = self.sequence.first_mut() {
      first.play();
    }
  }
}

impl<A, T> Accumulator<A> for Sequence<A, T>
where
  T: Clone,
  A: Copy + Default + PartialEq,
{
  fn accumulate(&mut self, delta: A) -> AccumulationResult<A> {
    let mut result = AccumulationResult::default();
    if !self.active || self.is_complete() {
      return result;
    }

    // Time left over when a child completes carries into the next one, so a single delta may complete several
    // children, or several cycles
    let mut delta = delta;
    let mut cycle_start_delta = delta;

    while let Some(current_anim) = self.sequence.get_mut(self.current_anim_index) {
      let child = current_anim.accumulate(delta);
      if !current_anim.is_complete() {
        break;
      }

      delta = child.remainder;
      self.current_anim_index += 1;

      if self.current_anim_index >= self.sequence.len() {
        result.completed_cycle = true;
        if self.cycle != Cycle::Forever {
          self.cycle_count = self.cycle_count.saturating_add(1);
        }

        // Hold on the last child so `sample()` keeps returning its final value
        if self.is_complete() {
          self.current_anim_index = self.sequence.len() - 1;
          result.remainder = delta;
          break;
        }

        // A cycle that took no time (e.g. every child is zero-length) would otherwise repeat forever
        let no_progress = delta == cycle_start_delta;
        cycle_start_delta = delta;

        self.current_anim_index = 0;
        self.reset_all();

        if no_progress {
          break;
        }
      }

      self.sequence[self.current_anim_index].play();
    }

    result
  }

  fn force(&mut self, current: A) {
    // replay the sequence up to the current value, even if it isn't playing
    let active = self.active;
    self.reset();
    self.active = true;
    self.accumulate(current);
    self.active = active;
  }

  fn is_complete(&self) -> bool {
    match self.cycle {
      Cycle::Once => self.cycle_count >= 1,
      Cycle::Times(n) => self.cycle_count >= n,
      Cycle::Forever => false,
    }
  }

  fn reset(&mut self) {
    self.current_anim_index = 0;
    self.cycle_count = 0;
    self.reset_all();
  }
}

impl<A, T> Animation<A, T> for Sequence<A, T>
where
  T: Clone,
  A: Copy + Default + PartialEq,
{
  fn sample(&self) -> T {
    self.sequence[self.current_anim_index].sample()
  }

  fn stop(&mut self) {
    self.active = false;
  }

  fn play(&mut self) {
    self.active = true;

    if let Some(current_anim) = self.sequence.get_mut(self.current_anim_index) {
      current_anim.play();
    }
  }

  fn active(&self) -> bool {
    self.active
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Two one-second steps: 0 -> 10, then 10 -> 20
  fn two_steps(cycle: Cycle) -> Sequence<f32, f32> {
    Sequence::new(
      vec![
        Tween::boxed(1.0, Curve::Linear, vec![0.0, 10.0], Cycle::Once),
        Tween::boxed(1.0, Curve::Linear, vec![10.0, 20.0], Cycle::Once),
      ],
      cycle,
    )
  }

  #[test]
  #[should_panic(expected = "at least 1 animation")]
  fn test_empty_sequence_is_rejected() {
    Sequence::<f32, f32>::new(Vec::new(), Cycle::Once);
  }

  #[test]
  fn test_plays_children_in_order() {
    let mut seq = two_steps(Cycle::Once);
    assert_eq!(seq.sample(), 0.0);

    seq.accumulate(0.5);
    assert_eq!(seq.sample(), 5.0);

    seq.accumulate(1.0);
    assert_eq!(seq.sample(), 15.0);
  }

  #[test]
  fn test_once_holds_final_value() {
    let mut seq = two_steps(Cycle::Once);

    seq.accumulate(1.0);
    let result = seq.accumulate(1.0);
    assert_eq!(result.completed_cycle, true);
    assert!(seq.is_complete());
    assert_eq!(seq.sample(), 20.0);

    seq.accumulate(0.5);
    assert_eq!(seq.sample(), 20.0);
  }

  #[test]
  fn test_once_reports_remainder() {
    let mut seq = two_steps(Cycle::Once);

    let result = seq.accumulate(2.5);
    assert_eq!(result.completed_cycle, true);
    assert_eq!(result.remainder, 0.5);
    assert_eq!(seq.sample(), 20.0);
  }

  #[test]
  fn test_times_stops_after_n_cycles() {
    let mut seq = two_steps(Cycle::Times(2));

    let result = seq.accumulate(2.0);
    assert_eq!(result.completed_cycle, true);
    assert!(!seq.is_complete());

    seq.accumulate(0.5);
    assert_eq!(seq.sample(), 5.0);

    seq.accumulate(1.5);
    assert!(seq.is_complete());
    assert_eq!(seq.sample(), 20.0);
  }

  #[test]
  fn test_forever_loops() {
    let mut seq = two_steps(Cycle::Forever);

    let result = seq.accumulate(2.5);
    assert_eq!(result.completed_cycle, true);
    assert_eq!(seq.sample(), 5.0);
    assert!(!seq.is_complete());
  }

  #[test]
  fn test_reset_keeps_playing() {
    let mut seq = two_steps(Cycle::Forever);

    seq.accumulate(1.5);
    seq.reset();
    assert_eq!(seq.sample(), 0.0);

    seq.accumulate(0.5);
    assert_eq!(seq.sample(), 5.0);
  }

  #[test]
  fn test_force() {
    let mut seq = two_steps(Cycle::Forever);

    seq.force(1.5);
    assert_eq!(seq.sample(), 15.0);

    seq.force(0.25);
    assert_eq!(seq.sample(), 2.5);
  }

  #[test]
  fn test_force_while_stopped_stays_stopped() {
    let mut seq = two_steps(Cycle::Forever).stopped();

    seq.force(1.5);
    assert_eq!(seq.sample(), 15.0);

    seq.accumulate(0.25);
    assert_eq!(seq.sample(), 15.0);
  }

  #[test]
  fn test_stop_pauses_mid_child() {
    let mut seq = two_steps(Cycle::Once);

    seq.accumulate(1.5);
    seq.stop();
    assert!(!seq.active());
    seq.accumulate(0.25);
    assert_eq!(seq.sample(), 15.0);

    seq.play();
    seq.accumulate(0.25);
    assert_eq!(seq.sample(), 17.5);
  }

  #[test]
  fn test_stopped_waits_for_play() {
    let mut seq = two_steps(Cycle::Once).stopped();

    seq.accumulate(0.5);
    assert_eq!(seq.sample(), 0.0);

    seq.play();
    seq.accumulate(0.5);
    assert_eq!(seq.sample(), 5.0);
  }

  #[test]
  fn test_reset_after_completing_replays() {
    let mut seq = two_steps(Cycle::Once);

    seq.accumulate(2.0);
    assert!(seq.is_complete());

    seq.reset();
    assert!(!seq.is_complete());
    seq.accumulate(1.5);
    assert_eq!(seq.sample(), 15.0);
  }

  #[test]
  fn test_nested_sequences() {
    let mut seq: Sequence<f32, f32> = Sequence::new(
      vec![
        Box::new(two_steps(Cycle::Once)),
        Tween::boxed(1.0, Curve::Linear, vec![20.0, 0.0], Cycle::Once),
      ],
      Cycle::Once,
    );

    seq.accumulate(2.5);
    assert_eq!(seq.sample(), 10.0);
  }

  #[test]
  fn test_zero_length_children_do_not_hang() {
    let mut seq: Sequence<f32, f32> = Sequence::new(
      vec![
        Tween::boxed(0.0, Curve::Linear, vec![0.0, 10.0], Cycle::Once),
        Tween::boxed(0.0, Curve::Linear, vec![10.0, 20.0], Cycle::Once),
      ],
      Cycle::Forever,
    );

    seq.accumulate(1.0);
  }
}
