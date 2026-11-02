use crate::prelude::GroupId;

/// Every system within Frontbox is always stored in a group with an ID. For fast lookup Frontbox often passes around
/// a `SystemHandle` which references the system ID and the ID of the parent group.
///
/// IDs alone don't identify a system: copies of a child system spawned into several groups (e.g. one per player) keep
/// the same ID, so two handles are the same system only if their groups match too.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SystemHandle {
  pub id: u64,
  pub group: GroupId,
}

impl SystemHandle {
  pub fn new(id: u64, group: GroupId) -> Self {
    Self { id, group }
  }
}
