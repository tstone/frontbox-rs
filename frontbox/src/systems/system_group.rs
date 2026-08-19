//! # System Groups
//! 
//! System groups allow a set of child systems to be run as a single unit, with some particular behaviors:
//! 
//! - **The system constraint of one-instance-per-type only applies within a group** -- For example, it might make sense to have a copy of `PlayerScoreSystem` _per player_. Normally only one of these could be spawned at the root. However, if a group was created per player, then one copy could be run per group.
//! 
//! - **Active state spans the entire group** -- Group can be made active or inactive, which cascades to all children. Like with scoring, this allows all systems for a player to be disabled at once. Each system can also manage their own active state, independent of the group's active state, but the group's active state is always a prerequisite (the logic for active is `group_active && system_active`).
//! 
//! - **System lookup also includes siblings** -- By default, performing `ctx.get::<T>` only looks at root systems. When a system is part of a group however, it will _also_ check for all sibling. In the case where the same system is running both within a group and at the root, priority is given to nearness. Siblings are searched first, then global.
//! 
//! Systems spawned into a group must implement `ChildSystem`, which requires that they be `Clone + Send + Sync`. If getting errors trying to add a child to a group, make sure to add `#[derive(Clone)]` to the system definition.
//! 
//! ```rust
//! # let ctx = Context
//! const group_name: &'static str = "example";
//! 
//! // Start an entire group of systems
//! ctx.spawn_system_group(group_name, vec![/* list of systems */]);
//! 
//! // Groups start deactivated by default
//! ctx.activate_system_group(group_name);
//! ctx.deactivate_system_group(group_name);
//! 
//! // The entire group can be despawned. All `on_shutdown` handlers will be invoked for child systems
//! ctx.despawn_system_group(group_name);
//! ```

use std::cell::RefMut;
use std::ops::{Deref, DerefMut};

use crate::prelude::*;

pub struct SystemGroup {
  pub(crate) systems: Systems,
  pub(crate) active: bool,
}

impl SystemGroup {
  pub fn new() -> Self {
    Self {
      systems: Systems::new(),
      active: true,
    }
  }

  pub fn child_ids(&self) -> Vec<&u64> {
    self.systems.ids()
  }

  pub fn get_by_id(&'_ self, system_id: &u64) -> Option<RefMut<'_, SystemContainer>> {
    self.systems.get_by_id(system_id)
  }

  pub fn activate(&mut self) {
    self.active = true;
  }

  pub fn deactivate(&mut self) {
    self.active = false;
  }
}

impl Deref for SystemGroup {
  type Target = Systems;

  fn deref(&self) -> &Self::Target {
    &self.systems
  }
}

impl DerefMut for SystemGroup {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.systems
  }
}
