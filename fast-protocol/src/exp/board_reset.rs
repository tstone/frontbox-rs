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

impl FastStringCommand for BoardResetCommand {
  fn to_string(&self) -> String {
    format!("BR@{:X}:\r", self.address)
  }
}

impl FastRequestCommand for BoardResetCommand {
  type Response = ProcessedResponse;

  fn prefix() -> &'static str {
    "br"
  }

  fn parse(&self, raw: RawResponse) -> Result<Self::Response, FastResponseError> {
    if raw.payload.to_lowercase() == "p" {
      Ok(ProcessedResponse::Processed)
    } else {
      Err(FastResponseError::InvalidFormat)
    }
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
