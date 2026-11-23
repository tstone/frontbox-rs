use crate::prelude::*;

/// Runs several copies of one system type under one parent, such as a switch combo for each job. A parent can
/// otherwise only have one system of a type.
///
/// The copies are its children, each a system of its own: they get every event, keep their own timers, and decide
/// for themselves whether they're active. But they can't be reached by type: lookups (`ctx.systems.get` and the
/// like) don't find them, and routines don't hand off to them. That's what makes copies safe, as there's never a
/// question of which one a lookup meant. A copy can still look up other systems as usual.
///
/// Deactivating, reactivating, or despawning it does the same to all its copies. Unlike a [`Group`], it can't be
/// part of a template copied for each player.
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # struct ComboSystem;
/// # impl ComboSystem { fn new(switch: &'static str) -> Self { Self } }
/// # impl System for ComboSystem {}
/// # fn example(ctx: &SystemContext) {
/// ctx.systems.spawn(Multi::new(
///   "switch combos",
///   vec![ComboSystem::new("start"), ComboSystem::new("action")],
/// ));
/// # }
/// ```
pub struct Multi {
  name: &'static str,
  description: Option<&'static str>,
  /// Spawned with it, then empty
  copies: Vec<SpawnableSystemContainer>,
}

impl Multi {
  /// `name` is for display (logs, the console), and doesn't need to be unique
  pub fn new<T: SpawnableSystem + 'static>(name: &'static str, copies: Vec<T>) -> Self {
    Self {
      name,
      description: None,
      copies: copies
        .into_iter()
        .map(SpawnableSystemContainer::new)
        .collect(),
    }
  }

  /// What its copies do, for the console, in place of the description every `Multi` has
  pub fn described(mut self, description: &'static str) -> Self {
    self.description = Some(description);
    self
  }

  pub fn name(&self) -> &'static str {
    self.name
  }

  pub(crate) fn multi_description(&self) -> Option<&'static str> {
    self.description
  }

  pub(crate) fn take_copies(&mut self) -> Vec<SpawnableSystemContainer> {
    std::mem::take(&mut self.copies)
  }
}

impl System for Multi {
  fn description() -> Option<&'static str> {
    Some("Holds copies of one system, which can't be looked up")
  }
}
