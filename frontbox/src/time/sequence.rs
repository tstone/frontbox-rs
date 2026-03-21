use std::time::Duration;

use crate::time::*;

/// Plays a sequence of animations in order
#[derive(Clone)]
pub struct Sequence<T> {
  sequence: Vec<Box<dyn Animation<T>>>,
  current_anim_index: usize,
  cycle: AnimationCycle,
  cycle_count: u32,
}

impl<T> Sequence<T> {
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

impl<T> Tickable for Sequence<T>
where
  T: Clone + Default,
{
  fn tick(&mut self, delta_time: Duration) -> Duration {
    if let Some(current_anim) = &mut self.sequence.get_mut(self.current_anim_index) {
      let remainder = current_anim.tick(delta_time);

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
        return self.tick(remainder);
      }
    }

    Duration::ZERO
  }

  fn is_complete(&self) -> bool {
    match self.cycle {
      AnimationCycle::Once => self.cycle_count >= 1,
      AnimationCycle::Times(n) => self.cycle_count >= n,
      AnimationCycle::Forever => false,
    }
  }

  fn completed_this_tick(&self) -> bool {
    self.is_complete() && self.current_anim_index == 0
  }

  fn reset(&mut self) {
    self.current_anim_index = 0;
    self.cycle_count = 0;
    self.reset_anims();
  }
}

impl<T> Animation<T> for Sequence<T>
where
  T: Clone + Default,
{
  fn sample(&self) -> T {
    if let Some(current_anim) = &mut self.sequence.get(self.current_anim_index) {
      return current_anim.sample();
    }
    T::default()
  }
}
