use crate::prelude::*;

/// One index on an EXP board's LED port, which FAST writes as one RGB triplet
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LedAddress {
  pub exp: ExpAddress,
  /// The LED port on the board, 0-based. Only for display: `index` already counts from the start of the breakout.
  pub port: u8,
  pub index: u8,
}

impl LedAddress {
  pub fn new(exp: ExpAddress, port: u8, index: u8) -> Self {
    Self { exp, port, index }
  }

  pub fn board(&self) -> u8 {
    self.exp.board_address
  }

  pub fn breakout(&self) -> Option<u8> {
    self.exp.breakout
  }
}

impl std::fmt::Display for LedAddress {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "LED {} on port {} of board {:X}", self.index, self.port + 1, self.board())
  }
}
