use crate::protocol::fast_command::ProcessedResponse;
use crate::protocol::prelude::*;

pub struct ConfigureHardwareCommand {
  pub platform_id: u16,
  pub switch_reporting: Option<SwitchReporting>,
}

impl ConfigureHardwareCommand {
  pub fn new(platform_id: u16, switch_reporting: Option<SwitchReporting>) -> Self {
    ConfigureHardwareCommand {
      platform_id,
      switch_reporting,
    }
  }
}

impl FastCommand for ConfigureHardwareCommand {
  type Response = ProcessedResponse;

  fn prefix() -> &'static str {
    "ch"
  }

  fn to_string(&self) -> String {
    format!(
      "CH:{:04},{}\r",
      self.platform_id,
      *self
        .switch_reporting
        .as_ref()
        .unwrap_or(&SwitchReporting::None) as u8
    )
  }

  fn parse(&self, raw: RawResponse) -> Result<Self::Response, FastResponseError> {
    if raw.payload.to_lowercase() == "p" {
      Ok(ProcessedResponse::Processed)
    } else if raw.payload.to_lowercase() == "f" {
      Ok(ProcessedResponse::Failed)
    } else {
      Err(FastResponseError::InvalidFormat)
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub enum SwitchReporting {
  None = 0,
  Verbose = 1,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let cmd = ConfigureHardwareCommand::new(65, Some(SwitchReporting::Verbose));
    let result = cmd.to_string();
    assert_eq!(result, "CH:0065,1\r");
  }
}
