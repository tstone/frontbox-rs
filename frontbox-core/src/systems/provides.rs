//! Looking up a system by a trait it implements, rather than by its concrete type. A system lists the traits it can
//! be found by in [`System::provides`], and callers look it up with `ctx.systems.get_dyn::<dyn Trait>()`.

use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::sync::Arc;

use crate::prelude::*;

/// The traits a system of type `T` can be looked up by. Filled in by [`System::provides`].
pub struct Provides<T> {
  traits: Vec<ProvidedTrait>,
  _system: PhantomData<fn(T)>,
}

impl<T: System> Provides<T> {
  /// The traits `T` provides
  pub(crate) fn collect() -> ProvidedTraits {
    let mut provides = Self {
      traits: Vec::new(),
      _system: PhantomData,
    };
    T::provides(&mut provides);
    ProvidedTraits(provides.traits.into())
  }

  /// Makes the system reachable as `I`, a trait it implements. `cast` turns the system into one, and `|s| s` is
  /// enough:
  ///
  /// ```rust,ignore
  /// provides.add::<dyn BallManager>(|s| s);
  /// ```
  pub fn add<I: ?Sized + 'static>(&mut self, cast: fn(&mut T) -> &mut I) -> &mut Self {
    let cast: Cast<I> = Box::new(cast_fn(move |system| {
      let system = (system as &mut dyn Any)
        .downcast_mut::<T>()
        .expect("a system's provided traits belong to its own type");
      cast(system)
    }));
    self.traits.push(ProvidedTrait {
      type_id: TypeId::of::<I>(),
      cast: Box::new(cast),
    });
    self
  }
}

/// The traits a system provides, with their type erased so every container can hold them. Cloned along with a
/// [`ChildSystemContainer`].
#[derive(Clone)]
pub(crate) struct ProvidedTraits(Arc<[ProvidedTrait]>);

impl ProvidedTraits {
  pub(crate) fn type_ids(&self) -> impl Iterator<Item = TypeId> + '_ {
    self.0.iter().map(|provided| provided.type_id)
  }

  /// `system` as `I`, if it provides `I`. `system` must be the system these traits were collected from.
  pub(crate) fn cast_mut<'a, I: ?Sized + 'static>(
    &self,
    system: &'a mut (dyn System + 'static),
  ) -> Option<&'a mut I> {
    let provided = self.0.iter().find(|provided| provided.type_id == TypeId::of::<I>())?;
    let cast = provided
      .cast
      .downcast_ref::<Cast<I>>()
      .expect("a provided trait's cast matches its type id");
    Some(cast(system))
  }
}

struct ProvidedTrait {
  /// Of the trait object, e.g. `TypeId::of::<dyn BallManager>()`
  type_id: TypeId,
  /// A [`Cast`] to that trait
  cast: Box<dyn Any + Send + Sync>,
}

type Cast<I> = Box<dyn for<'a> Fn(&'a mut (dyn System + 'static)) -> &'a mut I + Send + Sync>;

/// Pins a closure to the signature of [`Cast`], whose returned borrow lasts as long as the one it's given. Without it,
/// the closure's lifetimes aren't inferred that way.
fn cast_fn<I: ?Sized, F>(f: F) -> F
where
  F: for<'a> Fn(&'a mut (dyn System + 'static)) -> &'a mut I,
{
  f
}

#[cfg(test)]
mod tests {
  use super::*;

  trait Counter {
    fn bump(&mut self) -> u32;
  }

  #[derive(Clone, Default)]
  struct Provider(u32);

  impl Counter for Provider {
    fn bump(&mut self) -> u32 {
      self.0 += 1;
      self.0
    }
  }

  impl System for Provider {
    fn provides(provides: &mut Provides<Self>) {
      provides.add::<dyn Counter>(|s| s);
    }
  }

  /// Implements `Counter`, but doesn't list it
  struct Unlisted;

  impl Counter for Unlisted {
    fn bump(&mut self) -> u32 {
      0
    }
  }

  impl System for Unlisted {}

  #[test]
  fn a_system_is_found_by_a_trait_it_provides() {
    let mut ctx = TestContext::default();
    ctx.insert_system(Provider::default());
    let sys = ctx.sys_ctx();

    assert_eq!(sys.systems.with_dyn::<dyn Counter, _>(|counter| counter.bump()), Some(1));
    assert_eq!(sys.systems.expect_dyn::<dyn Counter>().bump(), 2);
    // and still by its own type
    assert_eq!(sys.systems.get::<Provider>().map(|provider| provider.0), Some(2));
  }

  #[test]
  fn a_trait_not_listed_in_provides_is_not_found() {
    let mut ctx = TestContext::default();
    ctx.insert_system(Unlisted);

    assert!(ctx.sys_ctx().systems.get_dyn::<dyn Counter>().is_none());
  }

  #[test]
  fn a_systems_own_type_is_not_found_as_a_trait() {
    let mut ctx = TestContext::default();
    ctx.insert_system(Provider::default());

    assert!(ctx.sys_ctx().systems.get_dyn::<Provider>().is_none());
  }

  #[test]
  fn removing_an_older_provider_keeps_the_newer_one() {
    let mut systems = Systems::new();
    let older = SystemContainer::new(Provider(10));
    let older_id = older.id();
    systems.insert(older);
    systems.insert(Provider(20));

    systems.remove(older_id);

    let mut counter = systems.borrow_by_trait::<dyn Counter>().unwrap().unwrap();
    assert_eq!(counter.bump(), 21);
  }

  #[test]
  fn removing_the_provider_removes_the_trait() {
    let mut systems = Systems::new();
    let provider = SystemContainer::new(Provider::default());
    let id = provider.id();
    systems.insert(provider);

    systems.remove(id);

    assert!(systems.borrow_by_trait::<dyn Counter>().unwrap().is_none());
  }

  #[test]
  fn spawnable_and_child_systems_keep_their_traits() {
    let mut systems = Systems::new();
    systems.insert(SpawnableSystemContainer::new(Provider(10)).to_system_container());
    let template = ChildSystemContainer::new(Provider(20));
    systems.insert(template.clone().to_system_container());

    // the latest inserted wins
    let mut counter = systems.borrow_by_trait::<dyn Counter>().unwrap().unwrap();
    assert_eq!(counter.bump(), 21);
  }
}
