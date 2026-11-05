#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Cycle {
  /// Run once
  #[default]
  Once,
  /// Loop exactly n times
  Times(u32),
  /// Loop continually
  Forever,
}
