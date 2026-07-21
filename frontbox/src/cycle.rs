#[derive(Debug, Clone, Copy)]
pub enum Cycle {
  Once,
  Times(u32),
  Forever,
}
