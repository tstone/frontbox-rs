use crate::animation::Accumulator;

/// Describes any value that can be changed over time. More specifically, an animation is a Tickable (something which can be marched forward with time) that returns a value.
pub trait Animation<Acc, Val>: Accumulator<Acc> {
  fn sample(&self) -> Val;
}

dyn_clone::clone_trait_object!(<A, T> Animation<A, T>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimationCycle {
  Once,
  Times(u32),
  Forever,
}
