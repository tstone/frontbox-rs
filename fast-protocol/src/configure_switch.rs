use crate::*;
use std::time::Duration;

pub struct ConfigureSwitchCommand {
  switch_id: usize,
  reporting: SwitchReportingMode,
  debounce_close: Option<Duration>,
  debounce_open: Option<Duration>,
}

impl ConfigureSwitchCommand {
  pub fn new(
    switch_id: usize,
    reporting: SwitchReportingMode,
    debounce_close: Option<Duration>,
    debounce_open: Option<Duration>,
  ) -> Self {
    ConfigureSwitchCommand {
      switch_id,
      reporting,
      debounce_close,
      debounce_open,
    }
  }
}

impl FastCommand for ConfigureSwitchCommand {
  type Response = ProcessedResponse;

  fn prefix() -> &'static str {
    "sl"
  }

  fn to_string(&self) -> String {
    // https://fastpinball.com/fast-serial-protocol/net/sl/
    format!(
      "SL:{:X},{},{:X},{:X}\r",
      self.switch_id,
      self.reporting as u8,
      self
        .debounce_close
        .unwrap_or(Duration::from_millis(2))
        .as_millis(),
      self
        .debounce_open
        .unwrap_or(Duration::from_millis(20))
        .as_millis()
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SwitchReportingMode {
  None = 0,
  ReportNormal = 1,
  ReportInverted = 2,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let result =
      ConfigureSwitchCommand::new(10, SwitchReportingMode::ReportNormal, None, None).to_string();
    assert_eq!(result, "SL:A,1,2,14\r");
  }
}
