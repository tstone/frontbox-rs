use crate::prelude::*;

/// Reports a system's status with [`SystemContext::report_status`] only when it differs from the last report, so a
/// system can call it at the end of every handler that might change its status. Without a tracer, the status isn't
/// built.
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// #[derive(serde::Serialize, PartialEq)]
/// struct Progress {
///   targets_hit: u8,
/// }
///
/// struct TargetsSystem {
///   targets_hit: u8,
///   reported: StatusReporter<Progress>,
/// }
///
/// impl TargetsSystem {
///   fn report(&mut self, ctx: &SystemContext) {
///     let targets_hit = self.targets_hit;
///     self.reported.report(ctx, || Progress { targets_hit });
///   }
/// }
/// ```
#[derive(Clone)]
pub struct StatusReporter<T> {
  last: Option<T>,
}

impl<T: serde::Serialize + PartialEq> StatusReporter<T> {
  pub fn new() -> Self {
    Self { last: None }
  }

  /// Builds the status with `status` and reports it, unless it's the same as last time or no tracer is attached
  pub fn report(&mut self, ctx: &SystemContext, status: impl FnOnce() -> T) {
    if !ctx.is_traced() {
      return;
    }
    let status = status();
    if self.last.as_ref() == Some(&status) {
      return;
    }
    ctx.report_status(&status);
    self.last = Some(status);
  }
}

impl<T: serde::Serialize + PartialEq> Default for StatusReporter<T> {
  fn default() -> Self {
    Self::new()
  }
}
