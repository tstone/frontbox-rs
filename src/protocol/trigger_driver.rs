use crate::protocol::prelude::*;

pub struct TriggerDriverCommand {
  driver_id: usize,
  control_mode: DriverTriggerControlMode,
  switch: Option<usize>,
}

impl TriggerDriverCommand {
  pub fn new(
    driver_id: usize,
    control_mode: DriverTriggerControlMode,
    switch: Option<usize>,
  ) -> Self {
    Self {
      driver_id,
      control_mode,
      switch,
    }
  }
}

impl FastCommand for TriggerDriverCommand {
  type Response = ProcessedResponse;

  fn prefix() -> &'static str {
    "tl"
  }

  fn to_string(&self) -> String {
    // https://fastpinball.com/fast-serial-protocol/net/tl/
    format!(
      "TL:{:X},{},{}\r",
      self.driver_id,
      self.control_mode as u8,
      self.switch.map_or("".to_string(), |s| format!("{:X}", s))
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
pub enum DriverTriggerControlMode {
  // Hardware-controlled. When the IO network senses the switch change, it will automatically fire the driver.
  Automatic = 0,
  // "Tap" (activate) the driver
  Manual = 1,
  // For "hold" modes turn the driver on
  On = 2,
  // For "hold" modes turn the driver off
  Off = 3,
}
