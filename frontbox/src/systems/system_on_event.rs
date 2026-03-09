use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use crate::prelude::*;

pub struct SystemOnEvent<E: FrontboxEvent> {
  pub method: Arc<Mutex<dyn FnMut(&mut Context) + Send>>,
  _phantom: PhantomData<E>,
}

impl<E: FrontboxEvent> Clone for SystemOnEvent<E> {
  fn clone(&self) -> Self {
    Self {
      method: Arc::clone(&self.method),
      _phantom: PhantomData,
    }
  }
}

impl<E: FrontboxEvent> SystemOnEvent<E> {
  /// This creates a system which receives a single event, E, and runs the given closure.
  ///
  /// Example:
  /// ```
  /// SystemOnEvent::<GameStarted>::new(|ctx| {
  ///   ctx.spawn_district("players", PlayerDistrict::new(vec![ /* ... */ ]));
  /// }),
  /// ```
  pub fn new(f: impl FnMut(&mut Context) + Send + 'static) -> Box<Self> {
    Box::new(Self {
      method: Arc::new(Mutex::new(f)),
      _phantom: PhantomData,
    })
  }
}

impl<E: FrontboxEvent + 'static> System for SystemOnEvent<E> {
  fn on_event(&mut self, event: &dyn FrontboxEvent, ctx: &mut Context) {
    handle_event!(event, {
        E => |_e| { (self.method.lock().unwrap())(ctx); }
    });
  }
}
