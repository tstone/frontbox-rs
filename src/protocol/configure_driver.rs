use crate::DriverPin;
use crate::hardware::driver_config::DriverConfig;
use crate::protocol::driver_trigger::DriverTriggerBuilder;

/// Configure a driver in Fast IO boards (DL)
/// https://fastpinball.com/fast-serial-protocol/net/dl/
pub fn request(driver: DriverPin, config: DriverConfig) -> String {
  match config {
    DriverConfig::Disabled => format!("DL:{:X},,,0\r", driver.id),
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
      driver.id,
      DriverTriggerBuilder::new()
        .enabled(true)
        .one_shot(true)
        .invert_switch1(invert_switch)
        .bits(),
      switch.map_or(0, |s| s.id),
      initial_pwm_length.as_millis(),
      initial_pwm_power,
      secondary_pwm_length.as_millis(),
      secondary_pwm_power,
      rest.as_millis()
    ),
    _ => todo!(),
  }
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use crate::Switch;
  use crate::hardware::power::Power;

  use super::*;

  #[test]
  fn test_pulse_driver() {
    let driver = DriverPin {
      id: 10,
      name: "Test Driver",
      parent_index: 0,
    };
    let config = DriverConfig::Pulse {
      switch: Some(Switch {
        id: 5,
        name: "Test Switch",
        parent_index: 0,
      }),
      invert_switch: Some(true),
      initial_pwm_length: Duration::from_millis(100),
      initial_pwm_power: Power::full(),
      secondary_pwm_length: Duration::from_millis(50),
      secondary_pwm_power: Power::percent(50),
      rest: Duration::from_millis(500),
    };
    let request_str = request(driver, config);
    assert_eq!(request_str, "DL:A,19,5,10,64,FF,32,7F,1F4\r");
  }
}
