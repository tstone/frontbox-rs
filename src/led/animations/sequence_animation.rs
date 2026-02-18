use std::time::Duration;

use crate::led::animation::{Animation, AnimationCycle};

/// Plays a sequence of animations in order
#[derive(Clone)]
pub struct SequenceAnimation<T> {
  sequence: Vec<Box<dyn Animation<T>>>,
  current_anim_index: usize,
  cycle: AnimationCycle,
  cycle_count: u32,
}

impl<T> SequenceAnimation<T> {
  pub fn new(sequence: Vec<Box<dyn Animation<T>>>, cycle: AnimationCycle) -> Box<Self> {
    Box::new(Self {
      sequence,
      current_anim_index: 0,
      cycle,
      cycle_count: 0,
    })
  }

  fn at_cycle_end(&self) -> bool {
    self.current_anim_index >= self.sequence.len()
  }
}

impl<T> Animation<T> for SequenceAnimation<T>
where
  T: Clone,
{
  fn tick(&mut self, delta_time: Duration) -> Duration {
    if self.current_anim_index >= self.sequence.len() {
      return delta_time;
    }

    let remainder = self.sequence[self.current_anim_index].tick(delta_time);
    if self.sequence[self.current_anim_index].is_complete() {
      self.current_anim_index += 1;

      if self.at_cycle_end() {
        self.cycle_count += 1;
      }

      if !self.is_complete() {
        // roll over extra time to next animation, if any
        return self.tick(remainder);
      }
    }

    Duration::ZERO
  }

  fn sample(&self) -> T {
    self.sequence[self.current_anim_index].sample()
  }

  fn is_complete(&self) -> bool {
    match self.cycle {
      AnimationCycle::Times(n) => self.cycle_count >= n && self.at_cycle_end(),
      AnimationCycle::Forever => false,
    }
  }
}
