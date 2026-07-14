use super::LedType;
use crate::common::{ProcessedResponse, expansion_addr};
use crate::error::FastResponseError;
use crate::raw_response::RawResponse;
use crate::{FastRequestCommand, FastStringCommand};

#[derive(Debug, Clone)]
pub struct ConfigureLedPortCommand {
  expansion_board: u8,
  breakout: Option<u8>,
  port: u8,
  led_type: LedType,
  start: u16,
  count: u8,
}

impl ConfigureLedPortCommand {
  pub fn new(
    expansion_board: u8,
    breakout: Option<u8>,
    port: u8,
    led_type: LedType,
    start: u16,
    count: u8,
  ) -> Self {
    Self {
      expansion_board,
      breakout,
      port,
      led_type,
      start,
      count,
    }
  }
}

impl FastStringCommand for ConfigureLedPortCommand {
  fn to_string(&self) -> String {
    let address = expansion_addr(self.expansion_board, self.breakout);
    format!(
      "ER@{}:{:X},{},{:02X},{:02X}\r",
      address,
      self.port,
      self.led_type.clone() as u8,
      self.start,
      self.count
    )
  }
}

impl FastRequestCommand for ConfigureLedPortCommand {
  type Response = ProcessedResponse;

  fn prefix() -> &'static str {
    "er"
  }

  fn parse(&self, raw: RawResponse) -> Result<Self::Response, FastResponseError> {
    ProcessedResponse::parse(raw)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let command = ConfigureLedPortCommand::new(1, Some(2), 3, LedType::SK6812, 4, 5);
    assert_eq!(command.to_string(), "ER@12:3,1,4,5\r");
  }
}
