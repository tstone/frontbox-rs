#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cycle {
  /// Run once
  Once,
  /// Loop exactly n times
  Times(u32),
  /// Loop continually
  Forever,
}
