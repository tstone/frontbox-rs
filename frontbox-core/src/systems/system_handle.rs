use crate::systems::GroupId;

/// Identifies a running system to the framework: its id, and where the run loop keeps it. Every system spawned gets
/// a new id, copies of a template included, so the id alone says which system it is.
///
/// A handle lets whoever holds it act as the system (e.g. `ServiceContext::for_system`), so only the framework creates
/// them, and they shouldn't be handed out in events. Pass [`id`](Self::id) to say who a system is. `Default` is a
/// placeholder for a system's field until it is spawned, and doesn't refer to a system.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SystemHandle {
  pub(crate) id: u64,
  /// The group holding the system: its parent's children, or the top level
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
