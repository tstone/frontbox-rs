use crate::plugins::Plugin;
use crate::prelude::*;

/// Provides support for LED declaration, layering, and conflict resolution by way of `LedSystem`
pub struct LedPlugin;

impl Plugin for LedPlugin {
  fn build(&self, app: &mut App) {
    app.system(LedSystem::new());
  }
}
