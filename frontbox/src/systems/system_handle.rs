/// Every system within Frontbox is always stored in a group with an ID. For fast lookup Frontbox often passes around
/// a `SystemHandle` which references the system ID and the key of the parent group.
///
/// IDs alone don't identify a system: copies of a child system spawned into several groups (e.g. one per player) keep
/// the same ID, so two handles are the same system only if their groups match too.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SystemHandle {
  pub id: u64,
  pub parent_key: &'static str,
}

impl SystemHandle {
  pub fn new(id: u64, parent_key: &'static str) -> Self {
    Self { id, parent_key }
  }
}
