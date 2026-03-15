use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use crate::prelude::*;

pub struct OnEventSystem<E: Event> {
  pub method: Arc<Mutex<dyn FnMut(&mut Context) + Send>>,
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
  /// SystemOnEvent::<GameStarted>::new(|ctx, cmds| {
  ///   cmds.spawn_system( ... );
  /// });
  /// ```
  pub fn new(f: impl FnMut(&mut Context) + Send + 'static) -> Box<Self> {
    Box::new(Self {
      method: Arc::new(Mutex::new(f)),
      _phantom: PhantomData,
    })
  }
}

impl<E: Event + 'static> ChildSystem for OnEventSystem<E> {
  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {
    handle_event!(event, {
        E => |_e| { (self.method.lock().unwrap())(ctx); }
    });
  }
}
