#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cycle {
  Once,
  Times(u32),
  Forever,
}
