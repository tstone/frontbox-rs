use crate::plugins::Watchdog;
use crate::prelude::*;

/// A plugin which registers minimal operational systems. You need this unless you want to manually
/// register or re-implement the fundamental systems of this plugin (e.g. the watchdog).
pub struct OperationalPlugin;

impl Plugin for OperationalPlugin {
  fn register(&self, app: &mut App) {
    app.system(Watchdog::new());
  }
}
