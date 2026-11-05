use crate::{Accumulator, Modulation};

/// Object-safe, cloneable extension of [`Modulation`], allowing modulations to be stored as `Box<dyn DynModulation>`.
pub trait DynModulation<S, A>: Modulation<S, A> + Accumulator<A> {
  fn play(&mut self);
  fn stop(&mut self);
}

dyn_clone::clone_trait_object!(<S, A> DynModulation<S, A>);
