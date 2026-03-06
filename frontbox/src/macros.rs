/// Dispatches an untyped `&dyn FrontboxEvent` to typed handler arms.
///
/// # Usage
/// ```ignore
/// fn on_event(&mut self, event: &dyn FrontboxEvent, _ctx: &mut Context) {
///     handle_event!(event, {
///         TimerComplete => |e| { self.on = !self.on; }
///         my_module::MyCustomEvent => |e| { self.handle_custom(e); }
///     });
/// }
/// ```
///
/// Each arm attempts a downcast; non-matching arms are skipped silently.
#[macro_export]
macro_rules! handle_event {
  ($event:expr, { $( $EventType:path => |$var:ident| $body:block )* }) => {
    $(
      if let Some($var) = $event.as_any().downcast_ref::<$EventType>() {
        $body
      }
    )*
  };
}
