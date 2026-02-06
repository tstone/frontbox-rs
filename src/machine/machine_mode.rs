#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineMode {
  Attract,
  Game,
  Admin,
}

impl PartialEq<MachineMode> for &MachineMode {
  fn eq(&self, other: &MachineMode) -> bool {
    **self == *other
  }
}
