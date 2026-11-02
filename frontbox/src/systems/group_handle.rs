/// A system group, as handed to the system that spawned it. Spawning a group is the only way to get one, and it's
/// what activating, deactivating, and despawning the group take, so only the spawning system (or whoever it passes the
/// handle to) can do those things.
///
/// Despawning takes the handle by value, so it can't be used again afterwards. Groups are despawned along with the
/// system that owns them, so an owner never holds the handle of a group that's gone.
///
/// A clone of the handle passed to another system can outlive the group, though, and if the owner spawns a new group
/// under the same name, the clone refers to that one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupHandle {
  name: &'static str,
}

impl GroupHandle {
  pub(crate) fn new(name: &'static str) -> Self {
    Self { name }
  }

  pub fn name(&self) -> &'static str {
    self.name
  }
}
