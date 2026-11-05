use crate::prelude::GroupId;

/// Every system within Frontbox is always stored in a group with an ID. For fast lookup Frontbox often passes around
/// a `SystemHandle` which references the system ID and the ID of the parent group.
///
/// IDs alone don't identify a system: copies of a child system spawned into several groups (e.g. one per player) keep
/// the same ID, so two handles are the same system only if their groups match too.
///
/// A handle lets whoever holds it act as the system (e.g. `ServiceContext::for_system`), so only the framework creates
/// them, and they shouldn't be handed out in events. `Default` is a placeholder for a system's field until it is
/// spawned, and doesn't refer to a system.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SystemHandle {
  pub(crate) id: u64,
  pub(crate) group: GroupId,
}

impl SystemHandle {
  pub(crate) fn new(id: u64, group: GroupId) -> Self {
    Self { id, group }
  }

  pub fn id(&self) -> u64 {
    self.id
  }
}
