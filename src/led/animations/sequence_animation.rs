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

  fn reset_anims(&mut self) {
    for anim in &mut self.sequence {
      anim.reset();
    }
  }
}

impl<T> Animation<T> for SequenceAnimation<T>
where
  T: Clone,
{
  fn tick(&mut self, delta_time: Duration) -> Duration {
    let remainder = self.sequence[self.current_anim_index].tick(delta_time);
    log::trace!(
      "SequenceAnimation tick: anim_index={}, cycle_count={}, remainder={:?}",
      self.current_anim_index,
      self.cycle_count,
      remainder
    );

    if self.sequence[self.current_anim_index].is_complete() {
      self.current_anim_index += 1;

      if self.current_anim_index >= self.sequence.len() {
        if self.cycle != AnimationCycle::Forever && self.cycle_count < u32::MAX {
          self.cycle_count += 1;
        }
        self.current_anim_index = 0;
        self.reset_anims();
      }

      // roll over extra time to next animation, if any
      return self.tick(remainder);
    }

    Duration::ZERO
  }

  fn sample(&self) -> T {
    self.sequence[self.current_anim_index].sample()
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
