use crate::animation::Accumulator;

/// Describes any value that can be changed over time. More specifically, an animation is a Tickable (something which can be marched forward with time) that returns a value.
pub trait Animation<Acc, Val>: Accumulator<Acc> {
  fn sample(&self) -> Val;

  fn play(&mut self);
  fn pause(&mut self);

  fn stop(&mut self) {
    self.reset();
    self.pause();
  }
}

dyn_clone::clone_trait_object!(<A, T> Animation<A, T>);
