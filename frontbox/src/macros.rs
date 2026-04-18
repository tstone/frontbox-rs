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
      fn on_spawn(&mut self, ctx: &Context) {
        self.$field.on_spawn(ctx)
      }
      fn on_deactivate(&mut self, ctx: &Context) {
        self.$field.on_deactivate(ctx)
      }
      fn on_reactivate(&mut self, ctx: &Context) {
        self.$field.on_reactivate(ctx)
      }
      fn on_despawn(&mut self, ctx: &Context) {
        self.$field.on_despawn(ctx)
      }
      fn on_tick(&mut self, delta: Duration, ctx: &Context) {
        self.$field.on_tick(delta, ctx)
      }
      fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
        self.$field.on_event(event, ctx)
      }
      fn on_interrupt(&mut self, event: &dyn Event, ctx: &Context) -> InterruptResult {
        self.$field.on_interrupt(event, ctx)
      }
      fn is_active(&self, ctx: &Context) -> bool {
        self.$field.is_active(ctx)
      }
    }
  };
}
