use std::sync::atomic::{AtomicU64, Ordering};

/// A system group, as handed to the system that spawned it. Spawning a group is the only way to get one, and it's
/// what activating, deactivating, and despawning the group take, so only the spawning system (or whoever it passes the
/// handle to) can do those things.
///
/// Despawning takes the handle by value, so it can't be used again afterwards. Groups are despawned along with the
/// system that owns them, so an owner never holds the handle of a group that's gone. A clone passed to another system
/// can outlive the group, but group IDs are never reused, so it can't reach any other group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupHandle {
  id: GroupId,
  name: &'static str,
}

impl GroupHandle {
  pub(crate) fn new(id: GroupId, name: &'static str) -> Self {
    Self { id, name }
  }

  pub fn id(&self) -> GroupId {
    self.id
  }

  /// The name the group was spawned with, for display. Names don't need to be unique.
  pub fn name(&self) -> &'static str {
    self.name
  }
}

static NEXT_GROUP_ID: AtomicU64 = AtomicU64::new(1);

/// Identifies a system group. Each group spawned gets a new one, never reused. The root is `GroupId::ROOT`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GroupId(u64);

impl GroupId {
  pub const ROOT: GroupId = GroupId(0);

  pub(crate) fn next() -> Self {
    Self(NEXT_GROUP_ID.fetch_add(1, Ordering::Relaxed))
  }
}

impl std::fmt::Display for GroupId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}
