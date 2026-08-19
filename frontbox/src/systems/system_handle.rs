/// Every system within Frontbox is always stored in a group with an ID. IDs are unique across all groups,
/// but for fast lookup Frontbox often passes around a `SystemHandle` which references the system ID and
/// the key of the parent group.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemHandle {
  pub id: u64,
  pub parent_key: &'static str,
}

impl SystemHandle {
  pub fn new(id: u64, parent_key: &'static str) -> Self {
    Self { id, parent_key }
  }
}
