use crate::protocol::common::expansion_addr;
use crate::protocol::prelude::*;

pub struct IdentifyHardwareCommand {
  expansion_board: u8,
  breakout: Option<u8>,
}

impl IdentifyHardwareCommand {
  pub fn new(expansion_board: u8, breakout: Option<u8>) -> Self {
    Self {
      expansion_board,
      breakout,
    }
  }
}

impl FastCommand for IdentifyHardwareCommand {
  type Response = ProcessedResponse;

  fn prefix() -> &'static str {
    "ih"
  }

  fn to_string(&self) -> String {
    let address = expansion_addr(self.expansion_board, self.breakout);
    format!("IH@{}\r", address)
  }

  fn parse(&self, raw: RawResponse) -> Result<Self::Response, FastResponseError> {
    log::trace!("Parsing IdentifyHardware response: {:?}", raw);
    Err(FastResponseError::InvalidFormat)
  }
}
