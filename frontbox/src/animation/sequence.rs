use crate::animation::*;

/// Plays a sequence of animations in order
#[derive(Clone)]
pub struct Sequence<A, T> {
  sequence: Vec<Box<dyn Animation<A, T>>>,
  current_anim_index: usize,
  cycle: AnimationCycle,
  cycle_count: u32,
}

impl<A, T> Sequence<A, T> {
  pub fn new(sequence: Vec<Box<dyn Animation<A, T>>>, cycle: AnimationCycle) -> Box<Self> {
    Box::new(Self {
      sequence,
      current_anim_index: 0,
      cycle,
      cycle_count: 0,
    })
  }

  fn reset_anims(&mut self) {
    for anim in &mut self.sequence {
      anim.reset();
    }
  }
}

impl<A, T> Accumulator<A> for Sequence<A, T>
where
  T: Clone + Default,
  A: Copy + Default,
{
  fn accumulate(&mut self, delta: A) -> AccumulationResult<A> {
    if let Some(current_anim) = &mut self.sequence.get_mut(self.current_anim_index) {
      let result = current_anim.accumulate(delta);

      if current_anim.is_complete() {
        self.current_anim_index += 1;

        if self.current_anim_index >= self.sequence.len() {
          if self.cycle != AnimationCycle::Forever && self.cycle_count < u32::MAX {
            self.cycle_count += 1;
          }
          self.current_anim_index = 0;
          self.reset_anims();
        }

        // roll over extra time to next animation, if any
        return self.accumulate(result.remainder);
      }
    }

    AccumulationResult::default()
  }

  fn set(&mut self, current: A) {
    // replay the sequence up to the current value
    self.reset();
    let mut remainder = current;
    for anim in &mut self.sequence {
      anim.set(current);
      remainder = anim.accumulate(remainder).remainder;
    }
  }

  fn is_complete(&self) -> bool {
    match self.cycle {
      AnimationCycle::Once => self.cycle_count >= 1,
      AnimationCycle::Times(n) => self.cycle_count >= n,
      AnimationCycle::Forever => false,
    }
  }

  fn reset(&mut self) {
    self.current_anim_index = 0;
    self.cycle_count = 0;
    self.reset_anims();
  }
}

impl<A, T> Animation<A, T> for Sequence<A, T>
where
  T: Clone + Default,
  A: Copy + Default,
{
  fn sample(&self) -> T {
    if let Some(current_anim) = &mut self.sequence.get(self.current_anim_index) {
      return current_anim.sample();
    }
    T::default()
  }
}
