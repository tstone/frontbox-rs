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

impl FastStringDispatch for ConfigureDriverCommand {
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
      DriverConfig::PulseHoldExtension {
        switch,
        invert_switch,
        initial_pwm_length,
        initial_pwm_power,
        secondary_pwm_length,
        secondary_pwm_power,
        rest,
      } => format!(
        "DL:{:X},{:X},{:X},78,{},{:X},{},{:X},{}\r",
        self.driver_id,
        DriverTriggerBuilder::new()
          .invert_switch1(invert_switch)
          .bits(),
        switch.unwrap_or(0),
        fast_ms_byte(initial_pwm_length),
        initial_pwm_power,
        fast_unit_ms_byte(secondary_pwm_length, 10),
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
      DriverConfig::FlipperMain3Try {
        button_switch,
        invert_button_switch,
        eos_switch,
        max_on_time,
        eos_hold_pwm_power,
        rest,
      } => {
        format!(
          "DL:{:X},{:X},{:X},5F,{:X},{},{:X},{},\r",
          self.driver_id,
          DriverTriggerBuilder::new()
            .invert_switch1(invert_button_switch)
            .bits(),
          button_switch,
          eos_switch,
          fast_ms_byte(max_on_time),
          eos_hold_pwm_power,
          fast_ms_byte(rest)
        )
      }
    }
  }
}

impl FastCommand for ConfigureDriverCommand {
  fn prefix(&self) -> &'static str {
    "dl"
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
    assert_eq!(request_str, "DL:A,91,5,10,64,FF,32,AA,FF\r");
  }

  #[test]
  fn test_pulse_hold_extension_driver() {
    let config = DriverConfig::PulseHoldExtension {
      switch: Some(0x12),
      invert_switch: None,
      initial_pwm_length: Duration::from_millis(30),
      initial_pwm_power: Power::FULL,
      secondary_pwm_length: Duration::from_millis(1500),
      secondary_pwm_power: Power::QUARTER,
      rest: Duration::from_millis(16),
    };
    let request_str = ConfigureDriverCommand::new(3, config).to_string();
    // <driver>,<trigger>,<switch>,78,<initial ms>,<initial pwm>,<hold x10ms>,<hold pwm>,<rest ms>
    assert_eq!(request_str, "DL:3,81,12,78,1E,FF,96,44,10\r");
  }

  #[test]
  fn test_pulse_hold_driver() {
    let config = DriverConfig::PulseHold {
      switch: Some(2),
      invert_switch: None,
      initial_pwm_length: Duration::from_millis(30),
      initial_pwm_power: Power::FULL,
      secondary_pwm_power: Power::QUARTER,
      rest: Duration::from_millis(16),
    };
    let request_str = ConfigureDriverCommand::new(1, config).to_string();
    // <driver>,<trigger>,<switch>,18,<pwm1 ms>,<pwm1>,<pwm2>,<rest ms>,<n/a>
    assert_eq!(request_str, "DL:1,81,2,18,1E,FF,44,10,\r");
  }

  #[test]
  fn test_pulse_hold_cancel_driver() {
    let config = DriverConfig::PulseHoldCancel {
      switch: Some(2),
      invert_switch: None,
      off_switch: Some(3),
      invert_off_switch: Some(true),
      initial_max_on_time: Duration::from_millis(30),
      initial_pwm_power: Power::FULL,
      secondary_pwm_power: Power::QUARTER,
      rest: Duration::from_millis(16),
    };
    let request_str = ConfigureDriverCommand::new(1, config).to_string();
    // <driver>,<trigger>,<on switch>,20,<off switch>,<pwm1 ms>,<pwm1>,<pwm2>,<rest ms>
    assert_eq!(request_str, "DL:1,A1,2,20,3,1E,FF,44,10\r");
  }

  #[test]
  fn test_delayed_pulse_driver() {
    let config = DriverConfig::DelayedPulse {
      switch: Some(2),
      invert_switch: None,
      delay_length: Duration::from_millis(50),
      initial_full_power_length: Duration::from_millis(20),
      secondary_pwm_power: Power::HALF,
      secondary_pwm_length: Duration::from_millis(10),
      rest: Duration::from_millis(16),
    };
    let request_str = ConfigureDriverCommand::new(1, config).to_string();
    // <driver>,<trigger>,<switch>,30,<delay x10ms>,<full power ms>,<pwm1 ms>,<pwm1>,<rest ms>
    assert_eq!(request_str, "DL:1,81,2,30,05,14,0A,AA,10\r");
  }

  #[test]
  fn test_long_pulse_driver() {
    let config = DriverConfig::LongPulse {
      switch: None,
      invert_switch: None,
      initial_pwm_length: Duration::from_millis(30),
      initial_pwm_power: Power::FULL,
      secondary_pwm_length: Duration::from_millis(2000),
      secondary_pwm_power: Power::QUARTER,
      rest: Duration::from_millis(16),
    };
    let request_str = ConfigureDriverCommand::new(1, config).to_string();
    // <driver>,<trigger>,<switch>,70,<pwm1 ms>,<pwm1>,<pwm2 x100ms>,<pwm2>,<rest ms>
    assert_eq!(request_str, "DL:1,81,0,70,1E,FF,14,44,10\r");
  }

  #[test]
  fn test_pulse_cancel_driver() {
    let config = DriverConfig::PulseCancel {
      switch: Some(2),
      invert_switch: Some(true),
      off_switch: Some(3),
      invert_off_switch: None,
      initial_full_power_length: Duration::from_millis(30),
      secondary_power_length: Duration::from_millis(2000),
      secondary_pwm_power: Power::QUARTER,
      rest: Duration::from_millis(16),
    };
    let request_str = ConfigureDriverCommand::new(1, config).to_string();
    // <driver>,<trigger>,<on switch>,75,<off switch>,<full power ms>,<pwm2 x100ms>,<pwm2>,<rest ms>
    assert_eq!(request_str, "DL:1,91,2,75,3,1E,14,44,10\r");
  }
}
