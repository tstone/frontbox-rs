use crate::prelude::*;

/// A system that only holds other systems: its children, spawned with it. Deactivating, reactivating, or despawning
/// it does the same to all of them, so it's how a set of systems is switched on and off together, such as the systems
/// for one player.
///
/// It's the one system that can run more than once with the same parent, so a game manager can spawn one for each
/// player, each with its own copy of the player's systems. Each copy finds its own siblings first (see
/// [lookups](mod@crate::systems#lookups)), so the same systems run in each without knowing which player they're for.
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # #[derive(Clone)]
/// # struct ModeSystem;
/// # impl System for ModeSystem {}
/// # fn example(ctx: &SystemContext) {
/// let player = ctx
///   .systems
///   .spawn(Group::new("player1", systems![ModeSystem]).described("Player 1's modes"));
///
/// ctx.systems.deactivate(player);
/// ctx.systems.activate(player);
/// ctx.systems.despawn(player);
/// # }
/// ```
#[derive(Clone)]
pub struct Group {
  name: &'static str,
  description: Option<&'static str>,
  /// Spawned with the group, then empty
  children: Vec<ChildSystemContainer>,
}

impl Group {
  /// `name` is for display (logs, the console), and doesn't need to be unique
  pub fn new(name: &'static str, children: Vec<ChildSystemContainer>) -> Self {
    Self {
      name,
      description: None,
      children,
    }
  }

  /// What the group holds, for the console, in place of the description every group has
  pub fn described(mut self, description: &'static str) -> Self {
    self.description = Some(description);
    self
  }

  pub fn name(&self) -> &'static str {
    self.name
  }

  pub(crate) fn group_description(&self) -> Option<&'static str> {
    self.description
  }

  pub(crate) fn take_children(&mut self) -> Vec<ChildSystemContainer> {
    std::mem::take(&mut self.children)
  }
}

impl System for Group {
  fn description() -> Option<&'static str> {
    Some("Holds systems that are switched on and off together")
  }
}
