use crate::*;

#[derive(Debug, Clone)]
pub struct BoardResetCommand {
  address: u8,
}

impl BoardResetCommand {
  pub fn new(address: u8) -> Self {
    Self { address }
  }
}

impl FastStringDispatch for BoardResetCommand {
  fn to_string(&self) -> String {
    format!("BR@{:X}:\r", self.address)
  }
}

impl FastCommand for BoardResetCommand {
  fn prefix(&self) -> &'static str {
    "br"
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let command = BoardResetCommand::new(0x1A);
    assert_eq!(command.to_string(), "BR@1A:\r");
  }
}
