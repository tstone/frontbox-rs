use std::fmt;

/// An address on the expansion network: a board, and optionally one of its breakouts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ExpAddress {
  pub board_address: u8,
  pub breakout: Option<u8>,
}

impl ExpAddress {
  pub fn new(board_address: u8, breakout: Option<u8>) -> Self {
    Self {
      board_address,
      breakout,
    }
  }
}

/// Formats as it's written in a command: the board in hex, followed by the breakout nibble if there is one
impl fmt::Display for ExpAddress {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{:X}", self.board_address)?;
    if let Some(breakout) = self.breakout {
      write!(f, "{:X}", breakout)?;
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn formats_with_and_without_breakout() {
    assert_eq!(ExpAddress::new(0x48, None).to_string(), "48");
    assert_eq!(ExpAddress::new(0x48, Some(1)).to_string(), "481");
  }
}
