use crate::protocol::prelude::driver_trigger::*;
use crate::protocol::prelude::*;

/// Configure a driver in Fast IO boards (DL)
pub struct ConfigureDriverCommand<'a> {
  driver_id: &'a usize,
  config: &'a DriverConfig,
}

impl ConfigureDriverCommand<'_> {
  pub fn new<'a>(driver_id: &'a usize, config: &'a DriverConfig) -> ConfigureDriverCommand<'a> {
    ConfigureDriverCommand { driver_id, config }
  }
}

impl FastCommand for ConfigureDriverCommand<'_> {
  type Response = ProcessedResponse;

  fn prefix() -> &'static str {
    "dl"
  }

  fn to_string(&self) -> String {
    // https://fastpinball.com/fast-serial-protocol/net/dl/
    match self.config {
      DriverConfig::Disabled => format!("DL:{:X},,,0\r", self.driver_id),
      DriverConfig::Pulse {
        switch,
        invert_switch,
        initial_pwm_length,
        initial_pwm_power,
        secondary_pwm_length,
        secondary_pwm_power,
        rest,
      } => format!(
        "DL:{:X},{:X},{:X},10,{:X},{:X},{:X},{:X},{:X}\r",
        self.driver_id,
        DriverTriggerBuilder::new()
          .enabled(true)
          .invert_switch1(invert_switch)
          .disable_switch(true)
          .bits(),
        switch.unwrap_or(0),
        initial_pwm_length.as_millis(),
        initial_pwm_power,
        secondary_pwm_length.as_millis(),
        secondary_pwm_power,
        rest.as_millis()
      ),
      _ => todo!(),
    }
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

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use super::*;

  #[test]
  fn test_pulse_driver() {
    let config = DriverConfig::Pulse {
      switch: Some(5),
      invert_switch: Some(true),
      initial_pwm_length: Duration::from_millis(100),
      initial_pwm_power: Power::full(),
      secondary_pwm_length: Duration::from_millis(50),
      secondary_pwm_power: Power::percent(50),
      rest: Duration::from_millis(500),
    };
    let request_str = ConfigureDriverCommand::new(&10, &config).to_string();
    assert_eq!(request_str, "DL:A,91,5,10,64,FF,32,7F,1F4\r");
  }
}
