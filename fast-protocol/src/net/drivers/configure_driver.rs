use std::time::Duration;

use crate::net::drivers::driver_trigger_builder::DriverTriggerBuilder;
use crate::net::prelude::*;
use crate::*;

/// Configure a driver in Fast IO boards (DL)
#[derive(Debug, Clone)]
pub struct ConfigureDriverCommand {
  driver_id: usize,
  config: DriverConfig,
}

impl ConfigureDriverCommand {
  pub fn new(driver_id: usize, config: DriverConfig) -> ConfigureDriverCommand {
    ConfigureDriverCommand { driver_id, config }
  }
}

// 1 ms tick (single byte, max 255)
pub fn fast_ms_byte(duration: Duration) -> String {
  let ms = duration.as_millis().min(255) as u8;
  format!("{:02X}", ms)
}

// ms tick multiplier (single byte)
pub fn fast_unit_ms_byte(duration: Duration, unit: u8) -> String {
  let ticks = (duration.as_millis() / unit as u128).min(255) as u8;
  format!("{:02X}", ticks)
}

impl FastStringCommand for ConfigureDriverCommand {
  fn to_string(&self) -> String {
    // https://fastpinball.com/fast-serial-protocol/net/dl/
    match self.config {
      DriverConfig::Disabled => format!("DL:{:X},80,0,0,0,0,0,0,0\r", self.driver_id),
      DriverConfig::Pulse {
        switch,
        invert_switch,
        initial_pwm_length,
        initial_pwm_power,
        secondary_pwm_length,
        secondary_pwm_power,
        rest,
      } => format!(
        "DL:{:X},{:X},{:X},10,{},{:X},{},{:X},{}\r",
        self.driver_id,
        DriverTriggerBuilder::new()
          .invert_switch1(invert_switch)
          .bits(),
        switch.unwrap_or(0),
        fast_ms_byte(initial_pwm_length),
        initial_pwm_power,
        fast_ms_byte(secondary_pwm_length),
        secondary_pwm_power,
        fast_ms_byte(rest)
      ),
      DriverConfig::PulseKick {
        switch,
        invert_switch,
        initial_pwm_length,
        initial_pwm_power,
        secondary_pwm_length,
        secondary_pwm_power,
        kick_length,
      } => format!(
        "DL:{:X},{:X},{:X},12,{},{:X},{},{:X},{}\r",
        self.driver_id,
        DriverTriggerBuilder::new()
          .invert_switch1(invert_switch)
          .bits(),
        switch.unwrap_or(0),
        fast_ms_byte(initial_pwm_length),
        initial_pwm_power,
        fast_ms_byte(secondary_pwm_length),
        secondary_pwm_power,
        fast_ms_byte(kick_length)
      ),
      DriverConfig::PulseHold {
        switch,
        invert_switch,
        initial_pwm_length,
        initial_pwm_power,
        secondary_pwm_power,
        rest,
      } => format!(
        "DL:{:X},{:X},{:X},18,{},{:X},{:X},{},\r",
        self.driver_id,
        DriverTriggerBuilder::new()
          .manual(switch.is_none())
          .invert_switch1(invert_switch)
          .bits(),
        switch.unwrap_or(0),
        fast_ms_byte(initial_pwm_length),
        initial_pwm_power,
        secondary_pwm_power,
        fast_ms_byte(rest)
      ),
      DriverConfig::PulseHoldCancel {
        switch,
        invert_switch,
        off_switch,
        invert_off_switch,
        initial_max_on_time: initial_pwm_length,
        initial_pwm_power,
        secondary_pwm_power,
        rest,
      } => format!(
        "DL:{:X},{:X},{:X},20,{:X},{},{:X},{:X},{}\r",
        self.driver_id,
        DriverTriggerBuilder::new()
          .invert_switch1(invert_switch)
          .invert_switch2(invert_off_switch)
          .bits(),
        switch.unwrap_or(0),
        off_switch.unwrap_or(0),
        fast_ms_byte(initial_pwm_length),
        initial_pwm_power,
        secondary_pwm_power,
        fast_ms_byte(rest)
      ),
      DriverConfig::DelayedPulse {
        switch,
        invert_switch,
        delay_length,
        initial_full_power_length,
        secondary_pwm_power,
        secondary_pwm_length,
        rest,
      } => format!(
        "DL:{:X},{:X},{:X},30,{},{},{},{:X},{}\r",
        self.driver_id,
        DriverTriggerBuilder::new()
          .invert_switch1(invert_switch)
          .bits(),
        switch.unwrap_or(0),
        fast_unit_ms_byte(delay_length, 10),
        fast_ms_byte(initial_full_power_length),
        fast_ms_byte(secondary_pwm_length),
        secondary_pwm_power,
        fast_ms_byte(rest)
      ),
      DriverConfig::LongPulse {
        switch,
        invert_switch,
        initial_pwm_length,
        initial_pwm_power,
        secondary_pwm_length,
        secondary_pwm_power,
        rest,
      } => format!(
        "DL:{:X},{:X},{:X},70,{},{:X},{},{:X},{}\r",
        self.driver_id,
        DriverTriggerBuilder::new()
          .invert_switch1(invert_switch)
          .bits(),
        switch.unwrap_or(0),
        fast_ms_byte(initial_pwm_length),
        initial_pwm_power,
        fast_unit_ms_byte(secondary_pwm_length, 100),
        secondary_pwm_power,
        fast_ms_byte(rest)
      ),
      DriverConfig::PulseCancel {
        switch,
        invert_switch,
        off_switch,
        invert_off_switch,
        initial_full_power_length: initial_power_length,
        secondary_power_length,
        secondary_pwm_power,
        rest,
      } => format!(
        "DL:{:X},{:X},{:X},75,{:X},{},{},{:X},{}\r",
        self.driver_id,
        DriverTriggerBuilder::new()
          .invert_switch1(invert_switch)
          .invert_switch2(invert_off_switch)
          .bits(),
        switch.unwrap_or(0),
        off_switch.unwrap_or(0),
        fast_ms_byte(initial_power_length),
        fast_unit_ms_byte(secondary_power_length, 100),
        secondary_pwm_power,
        fast_ms_byte(rest)
      ),
      DriverConfig::FlipperMainDirect {
        button_switch,
        invert_button_switch,
        eos_switch,
        initial_pwm_power,
        secondary_pwm_power,
        max_eos_time,
        next_flip_refresh,
      } => {
        format!(
          "DL:{:X},{:X},{:X},5E,{:X},{:X},{:X},{},{}\r",
          self.driver_id,
          DriverTriggerBuilder::new()
            .invert_switch1(invert_button_switch)
            .bits(),
          button_switch,
          eos_switch,
          initial_pwm_power,
          secondary_pwm_power,
          fast_ms_byte(max_eos_time),
          fast_ms_byte(next_flip_refresh)
        )
      }
      DriverConfig::FlipperHoldDirect {
        button_switch,
        invert_button_switch,
        driver_on_time,
        initial_pwm_power,
        secondary_pwm_power,
      } => {
        format!(
          "DL:{:X},{:X},{:X},5D,{},{:X},{:X},00,00\r",
          self.driver_id,
          DriverTriggerBuilder::new()
            .invert_switch1(invert_button_switch)
            .bits(),
          button_switch,
          fast_ms_byte(driver_on_time),
          initial_pwm_power,
          secondary_pwm_power,
        )
      }
    }
  }
}

impl FastRequestCommand for ConfigureDriverCommand {
  type Response = ProcessedResponse;

  fn prefix() -> &'static str {
    "dl"
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
  use super::*;
  use crate::Power;
  use std::time::Duration;

  #[test]
  fn test_pulse_driver() {
    let config = DriverConfig::Pulse {
      switch: Some(5),
      invert_switch: Some(true),
      initial_pwm_length: Duration::from_millis(100),
      initial_pwm_power: Power::FULL,
      secondary_pwm_length: Duration::from_millis(50),
      secondary_pwm_power: Power::HALF,
      rest: Duration::from_millis(500),
    };
    let request_str = ConfigureDriverCommand::new(10, config).to_string();
    assert_eq!(request_str, "DL:A,91,5,10,64,FF,32,7F,1F4\r");
  }
}
