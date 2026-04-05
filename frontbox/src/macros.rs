#[macro_export]
macro_rules! events {
  ($($signal:expr),* $(,)?) => {
    vec![$(Box::new($signal) as Box<dyn Event>),*]
  }
}

#[macro_export]
macro_rules! systems {
  ($($system:expr),* $(,)?) => {
    vec![$($system.into()),*]
  }
}

#[macro_export]
/// Implements System for the given type, assuming `inner` is the field name of the inner system
macro_rules! delegate_system {
  ($type:ty, $field:ident) => {
    impl System for $type {
      fn on_startup(&mut self, ctx: &Context, systems: &Systems) {
        self.$field.on_startup(ctx, systems)
      }
      fn on_deactivate(&mut self, ctx: &Context, systems: &Systems) {
        self.$field.on_deactivate(ctx, systems)
      }
      fn on_reactivate(&mut self, ctx: &Context, systems: &Systems) {
        self.$field.on_reactivate(ctx, systems)
      }
      fn on_shutdown(&mut self, ctx: &Context, systems: &Systems) {
        self.$field.on_shutdown(ctx, systems)
      }
      fn on_tick(&mut self, delta: Duration, ctx: &Context, systems: &Systems) {
        self.$field.on_tick(delta, ctx, systems)
      }
      fn on_event(&mut self, event: &dyn Event, ctx: &Context, systems: &Systems) {
        self.$field.on_event(event, ctx, systems)
      }
      fn on_interrupt(&mut self, event: &dyn Event, ctx: &Context) -> InterruptResult {
        self.$field.on_interrupt(event, ctx)
      }
      fn is_active(&self, ctx: &Context, systems: &Systems) -> bool {
        self.$field.is_active(ctx, systems)
      }
    }
  };
}
