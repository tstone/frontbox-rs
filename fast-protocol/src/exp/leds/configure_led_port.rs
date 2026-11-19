use super::LedType;
use crate::{ExpAddress, FastCommand, FastStringDispatch};

#[derive(Debug, Clone)]
pub struct ConfigureLedPortCommand {
  address: ExpAddress,
  port: u8,
  led_type: LedType,
  start: u8,
  count: u8,
}

impl ConfigureLedPortCommand {
  pub fn new(address: ExpAddress, port: u8, led_type: LedType, start: u8, count: u8) -> Self {
    Self {
      address,
      port,
      led_type,
      start,
      count,
    }
  }
}

impl FastStringDispatch for ConfigureLedPortCommand {
  fn to_string(&self) -> String {
    format!(
      "ER@{}:{:X},{},{:02X},{:02X}\r",
      self.address,
      self.port,
      self.led_type.clone() as u8,
      self.start,
      self.count
    )
  }
}

impl FastCommand for ConfigureLedPortCommand {
  fn prefix(&self) -> &'static str {
    "er"
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let command = ConfigureLedPortCommand::new(ExpAddress::new(1, Some(2)), 3, LedType::SK6812, 4, 5);
    assert_eq!(command.to_string(), "ER@12:3,1,04,05\r");
  }
}
