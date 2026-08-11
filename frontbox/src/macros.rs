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
      fn on_spawn(&mut self, ctx: &SystemContext) {
        self.$field.on_spawn(ctx)
      }
      fn on_deactivate(&mut self, ctx: &SystemContext) {
        self.$field.on_deactivate(ctx)
      }
      fn on_reactivate(&mut self, ctx: &SystemContext) {
        self.$field.on_reactivate(ctx)
      }
      fn on_despawn(&mut self, ctx: &SystemContext) {
        self.$field.on_despawn(ctx)
      }
      fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {
        self.$field.on_tick(delta, ctx)
      }
      fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
        self.$field.on_event(event, ctx)
      }
      fn on_interrupt(&mut self, event: &dyn Event, ctx: &SystemContext) -> InterruptResult {
        self.$field.on_interrupt(event, ctx)
      }
      fn is_active(&self, ctx: &SystemContext) -> bool {
        self.$field.is_active(ctx)
      }
    }
  };
}

/// Declares one or more hardware definitions as lazily-initialized statics.
/// Each entry is a builder chain (no `.build()` needed -- the macro adds it),
/// wrapped in a `LazyLock<T>` so it's usable as `&'static T` anywhere via deref.
#[macro_export]
macro_rules! hardware_defs {
    ($(
        $(#[$attr:meta])*
        $vis:vis $name:ident : $ty:ty = $builder:expr;
    )*) => {
        $(
            $(#[$attr])*
            $vis static $name: ::std::sync::LazyLock<$ty> =
                ::std::sync::LazyLock::new(|| $builder.build());
        )*
    };
}
