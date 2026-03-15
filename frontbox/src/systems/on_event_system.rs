use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use crate::prelude::*;

/// Create a new system from a closure, for a single event
pub struct OnEventSystem<E: Event> {
  pub method: Arc<Mutex<dyn FnMut(&E, &mut Context) + Send>>,
  _phantom: PhantomData<E>,
}

impl<E: Event> Clone for OnEventSystem<E> {
  fn clone(&self) -> Self {
    Self {
      method: Arc::clone(&self.method),
      _phantom: PhantomData,
    }
  }
}

impl<E: Event> OnEventSystem<E> {
  /// This creates a system which receives a single event, E, and runs the given closure.
  ///
  /// Example:
  /// ```ignore
  /// SystemOnEvent::<GameStarted>::new(|event, ctx| {
  ///   ctx.spawn_system( ... );
  /// });
  /// ```
  pub fn new(f: impl FnMut(&E, &mut Context) + Send + 'static) -> Box<Self> {
    Box::new(Self {
      method: Arc::new(Mutex::new(f)),
      _phantom: PhantomData,
    })
  }
}

impl<E: Event + 'static> ChildSystem for OnEventSystem<E> {
  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {
    if let Some(e) = event.downcast::<E>() {
      (self.method.lock().unwrap())(e, ctx);
    }
  }
}
