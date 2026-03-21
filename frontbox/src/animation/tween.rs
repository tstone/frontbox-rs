use std::ops::{AddAssign, SubAssign};

use crate::animation::*;

/// Animation implementation that interpolates (lerps) between two values of type T over a specified quatity using a given curve
#[derive(Clone)]
pub struct Tween<A: ToF32 + Copy + Default, T: Lerp + Clone + Send + Sync> {
  // the amount which signals this animation is done
  pub target: A,
  current: A,
  pub curve: Curve,
  pub stops: Vec<T>,
  pub cycle: AnimationCycle,
  cycle_count: u32,
  current_stop_index: usize,
}

impl<A, T> Tween<A, T>
where
  T: Lerp + Clone + Send + Sync,
  A: ToF32 + Copy + Default + AddAssign + SubAssign + PartialEq + Send + Sync,
{
  pub fn new(target: A, curve: Curve, stops: Vec<T>, cycle: AnimationCycle) -> Box<Self> {
    Box::new(Self {
      target,
      current: A::default(),
      curve,
      stops,
      cycle,
      cycle_count: 0,
      current_stop_index: 0,
    })
  }

  fn next_index(&self) -> usize {
    (self.current_stop_index + 1) % self.stops.len()
  }

  pub fn reverse(&mut self) {
    Tween {
      target: self.target,
      current: A::default(),
      curve: Curve::Reverse(Box::new(self.curve.clone())),
      stops: self.stops.clone().into_iter().rev().collect(),
      cycle: self.cycle.clone(),
      cycle_count: self.cycle_count,
      current_stop_index: self.current_stop_index,
    };
  }
}

impl<A, T> Accumulator<A> for Tween<A, T>
where
  T: Lerp + Clone + Send + Sync,
  A: ToF32 + Copy + Default + AddAssign + SubAssign + PartialOrd + PartialEq + Send + Sync,
{
  fn accumulate(&mut self, delta_time: A) -> AccumulationResult<A> {
    if self.is_complete() {
      return AccumulationResult::default();
    }

    self.current += delta_time;
    if self.current >= self.target {
      self.current -= self.target;
      let is_last_stop = self.current_stop_index == self.stops.len() - 2;

      let mut completed_just_now = false;
      let remainder = self.current;

      if is_last_stop {
        match self.cycle {
          AnimationCycle::Forever => {
            self.current_stop_index = 0;
          }
          AnimationCycle::Once => {
            self.cycle_count += 1;
            completed_just_now = true;
          }
          AnimationCycle::Times(n) => {
            self.cycle_count += 1;
            if self.cycle_count < n {
              self.current_stop_index = 0;
            } else {
              completed_just_now = true;
            }
          }
        }
      } else {
        self.current_stop_index += 1;
      }

      return AccumulationResult {
        completed_just_now,
        remainder,
      };
    }

    AccumulationResult::default()
  }

  fn is_complete(&self) -> bool {
    match self.cycle {
      AnimationCycle::Once => self.cycle_count > 0,
      AnimationCycle::Times(n) => self.cycle_count >= n,
      AnimationCycle::Forever => false,
    }
  }

  fn reset(&mut self) {
    self.current = A::default();
    self.cycle_count = 0;
    self.current_stop_index = 0;
  }

  fn set(&mut self, current: A) {
    self.current = current;
  }
}

impl<A, T> Animation<A, T> for Tween<A, T>
where
  T: Lerp + Clone + Send + Sync,
  A: ToF32 + Copy + Default + AddAssign + SubAssign + PartialEq + PartialOrd + Send + Sync,
{
  fn sample(&self) -> T {
    let phase = (self.current.to_f32() / self.target.to_f32()).min(1.0);
    let curve_value = self.curve.sample(phase);
    let from = &self.stops[self.current_stop_index];
    let to = &self.stops[self.next_index()];
    from.interpolate(to, curve_value)
  }
}
