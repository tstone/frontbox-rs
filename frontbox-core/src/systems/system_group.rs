//! How the run loop stores systems: each system's children are kept together, in a `SystemGroup` owned by that
//! system, and the top-level systems in the root's. Systems never see these; they work with [`Group`] and system
//! ids. See [the system tree](mod@crate::systems#the-system-tree).

use std::any::TypeId;
use std::cell::{Cell, RefMut};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::prelude::*;
use crate::systems::state::StoredState;

/// The children of one system, or the top-level systems
pub struct SystemGroup {
  pub(crate) systems: Systems,
  /// Whether the owner is active, so its children can be. Always true for the root.
  active: Cell<bool>,
  /// The system whose children these are. The root has none.
  pub(crate) owner: Option<SystemHandle>,
  /// Every state, kept in the root's. See [states](crate::systems::state)
  pub(crate) states: HashMap<TypeId, StoredState>,
  /// Whether these are a [`Multi`]'s copies. They're left out of the type table, so they can share a type, and
  /// lookups and routines don't find them.
  pub(crate) copies: bool,
}

impl SystemGroup {
  pub(crate) fn new() -> Self {
    Self {
      systems: Systems::new(),
      active: Cell::new(true),
      owner: None,
      states: HashMap::new(),
      copies: false,
    }
  }

  /// The children of `owner`. `copies` if the owner is a [`Multi`].
  pub(crate) fn owned_by(owner: SystemHandle, active: bool, copies: bool) -> Self {
    Self {
      owner: Some(owner),
      active: Cell::new(active),
      copies,
      ..Self::new()
    }
  }

  /// Add a system to the group, in the type table unless the group holds copies
  pub(crate) fn add(&mut self, system: SystemContainer) {
    match self.copies {
      true => self.systems.insert_unlisted(system),
      false => self.systems.insert(system),
    }
  }

  pub(crate) fn is_active(&self) -> bool {
    self.active.get()
  }

  pub(crate) fn set_active(&self, active: bool) {
    self.active.set(active);
  }

  pub fn get_by_id(&'_ self, system_id: &u64) -> Option<RefMut<'_, SystemContainer>> {
    self.systems.get_by_id(system_id)
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

static NEXT_GROUP_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Identifies a [`SystemGroup`]. Each one gets a new id, never reused. The root is `GroupId::ROOT`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct GroupId(u64);

impl GroupId {
  pub(crate) const ROOT: GroupId = GroupId(0);

  pub(crate) fn next() -> Self {
    Self(NEXT_GROUP_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
  }
}

impl std::fmt::Display for GroupId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}
