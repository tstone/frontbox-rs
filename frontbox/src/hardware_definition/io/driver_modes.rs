use std::collections::HashMap;
use std::time::Duration;

use fast_protocol::{DriverConfig, Power};

/// DriverMode is a wrapper around DriverConfig that allows these features:
/// 1. Referencing switches by name instead of index, which avoids having to calculate ID offsets
/// 2. Allows use of ..Default::default() since DriverConfig is an enum
pub trait DriverMode {
  fn to_config(&self, switch_lookup: &HashMap<&'static str, usize>) -> DriverConfig;
}

/// PulseMode -- https://fastpinball.com/fast-serial-protocol/net/driver-mode/10/
#[derive(Debug, Clone)]
pub struct PulseMode {
  pub switch: Option<&'static str>,
  pub invert_switch: Option<bool>,
  pub initial_pwm_length: Duration,
  pub initial_pwm_power: Power,
  pub secondary_pwm_length: Duration,
  pub secondary_pwm_power: Power,
  pub rest: Duration,
}

impl Default for PulseMode {
  fn default() -> Self {
    Self {
      switch: None,
      invert_switch: None,
      initial_pwm_length: Duration::from_millis(20),
      initial_pwm_power: Power::FULL,
      secondary_pwm_length: Duration::ZERO,
      secondary_pwm_power: Power::ZERO,
      rest: Duration::from_millis(80),
    }
  }
}

impl DriverMode for PulseMode {
  fn to_config(&self, switch_lookup: &HashMap<&'static str, usize>) -> DriverConfig {
    DriverConfig::Pulse {
      switch: self.switch.and_then(|s| switch_lookup.get(s).cloned()),
      invert_switch: self.invert_switch,
      initial_pwm_length: self.initial_pwm_length,
      initial_pwm_power: self.initial_pwm_power,
      secondary_pwm_length: self.secondary_pwm_length,
      secondary_pwm_power: self.secondary_pwm_power,
      rest: self.rest,
    }
  }
}

/// PulseKickMode -- https://fastpinball.com/fast-serial-protocol/net/driver-mode/12/
#[derive(Debug, Clone)]
pub struct PulseKickMode {
  pub switch: Option<&'static str>,
  pub invert_switch: Option<bool>,
  pub initial_pwm_length: Duration,
  pub initial_pwm_power: Power,
  pub secondary_pwm_length: Duration,
  pub secondary_pwm_power: Power,
  pub kick_length: Duration,
}

impl Default for PulseKickMode {
  fn default() -> Self {
    Self {
      switch: None,
      invert_switch: None,
      initial_pwm_length: Duration::from_millis(30),
      initial_pwm_power: Power::FULL,
      secondary_pwm_length: Duration::ZERO,
      secondary_pwm_power: Power::ZERO,
      kick_length: Duration::from_millis(500),
    }
  }
}

impl DriverMode for PulseKickMode {
  fn to_config(&self, switch_lookup: &HashMap<&'static str, usize>) -> DriverConfig {
    DriverConfig::PulseKick {
      switch: self.switch.and_then(|s| switch_lookup.get(s).cloned()),
      invert_switch: self.invert_switch,
      initial_pwm_length: self.initial_pwm_length,
      initial_pwm_power: self.initial_pwm_power,
      secondary_pwm_length: self.secondary_pwm_length,
      secondary_pwm_power: self.secondary_pwm_power,
      kick_length: self.kick_length,
    }
  }
}

/// PulseHoldMode -- https://fastpinball.com/fast-serial-protocol/net/driver-mode/18/
#[derive(Debug, Clone)]
pub struct PulseHoldMode {
  pub switch: Option<&'static str>,
  pub invert_switch: Option<bool>,
  pub initial_pwm_length: Duration,
  pub initial_pwm_power: Power,
  pub secondary_pwm_power: Power,
  pub rest: Duration,
}

impl Default for PulseHoldMode {
  fn default() -> Self {
    Self {
      switch: None,
      invert_switch: None,
      initial_pwm_length: Duration::from_millis(30),
      initial_pwm_power: Power::FULL,
      secondary_pwm_power: Power::ZERO,
      rest: Duration::ZERO,
    }
  }
}

impl DriverMode for PulseHoldMode {
  fn to_config(&self, switch_lookup: &HashMap<&'static str, usize>) -> DriverConfig {
    DriverConfig::PulseHold {
      switch: self.switch.and_then(|s| switch_lookup.get(s).cloned()),
      invert_switch: self.invert_switch,
      initial_pwm_length: self.initial_pwm_length,
      initial_pwm_power: self.initial_pwm_power,
      secondary_pwm_power: self.secondary_pwm_power,
      rest: self.rest,
    }
  }
}

/// PulseHoldCancelMode -- https://fastpinball.com/fast-serial-protocol/net/driver-mode/20/
#[derive(Debug, Clone)]
pub struct PulseHoldCancelMode {
  pub switch: Option<&'static str>,
  pub invert_switch: Option<bool>,
  pub off_switch: usize,
  pub invert_off_switch: bool,
  pub initial_pwm_length: Duration,
  pub secondary_pwm_power: Power,
  pub secondary_pwm_length: Duration,
  pub rest: Duration,
}

impl Default for PulseHoldCancelMode {
  fn default() -> Self {
    Self {
      switch: None,
      invert_switch: None,
      off_switch: 0,
      invert_off_switch: false,
      initial_pwm_length: Duration::from_millis(30),
      secondary_pwm_power: Power::percent(10),
      secondary_pwm_length: Duration::from_millis(500),
      rest: Duration::from_millis(500),
    }
  }
}

impl DriverMode for PulseHoldCancelMode {
  fn to_config(&self, switch_lookup: &HashMap<&'static str, usize>) -> DriverConfig {
    DriverConfig::PulseHoldCancel {
      switch: self.switch.and_then(|s| switch_lookup.get(s).cloned()),
      invert_switch: self.invert_switch,
      off_switch: self.off_switch,
      invert_off_switch: self.invert_off_switch,
      initial_pwm_length: self.initial_pwm_length,
      secondary_pwm_power: self.secondary_pwm_power,
      secondary_pwm_length: self.secondary_pwm_length,
      rest: self.rest,
    }
  }
}

/// LongPulseMode -- https://fastpinball.com/fast-serial-protocol/net/driver-mode/70/
#[derive(Debug, Clone)]
pub struct LongPulseMode {
  pub switch: Option<&'static str>,
  pub invert_switch: Option<bool>,
  pub initial_pwm_length: Duration,
  pub initial_pwm_power: Power,
  pub secondary_pwm_length: Duration,
  pub secondary_pwm_power: Power,
  pub rest: Duration,
}

impl Default for LongPulseMode {
  fn default() -> Self {
    Self {
      switch: None,
      invert_switch: None,
      initial_pwm_length: Duration::from_millis(200),
      initial_pwm_power: Power::FULL,
      secondary_pwm_length: Duration::from_millis(1000),
      secondary_pwm_power: Power::percent(25),
      rest: Duration::from_millis(1000),
    }
  }
}

impl DriverMode for LongPulseMode {
  fn to_config(&self, switch_lookup: &HashMap<&'static str, usize>) -> DriverConfig {
    DriverConfig::LongPulse {
      switch: self.switch.and_then(|s| switch_lookup.get(s).cloned()),
      invert_switch: self.invert_switch,
      initial_pwm_length: self.initial_pwm_length,
      initial_pwm_power: self.initial_pwm_power,
      secondary_pwm_length: self.secondary_pwm_length,
      secondary_pwm_power: self.secondary_pwm_power,
      rest: self.rest,
    }
  }
}

#[derive(Debug, Clone)]
pub struct FlipperMainDirectMode {
  pub button_switch: &'static str,
  pub invert_button_switch: Option<bool>,
  pub eos_switch: &'static str,
  pub initial_pwm_power: Power,
  pub secondary_pwm_power: Power,
  pub max_eos_time: Duration,
  pub next_flip_refresh: Duration,
}

impl Default for FlipperMainDirectMode {
  fn default() -> Self {
    Self {
      button_switch: "",
      invert_button_switch: None,
      eos_switch: "",
      initial_pwm_power: Power::percent(45),
      secondary_pwm_power: Power::FULL,
      max_eos_time: Duration::from_millis(60),
      next_flip_refresh: Duration::from_millis(8),
    }
  }
}

impl DriverMode for FlipperMainDirectMode {
  fn to_config(&self, switch_lookup: &HashMap<&'static str, usize>) -> DriverConfig {
    DriverConfig::FlipperMainDirect {
      button_switch: switch_lookup
        .get(self.button_switch)
        .cloned()
        .expect("Flipper main direct mode requires a valid button switch"),
      invert_button_switch: self.invert_button_switch,
      eos_switch: switch_lookup
        .get(self.eos_switch)
        .cloned()
        .expect("Flipper main direct mode requires a valid EOS switch"),
      initial_pwm_power: self.initial_pwm_power,
      secondary_pwm_power: self.secondary_pwm_power,
      max_eos_time: self.max_eos_time,
      next_flip_refresh: self.next_flip_refresh,
    }
  }
}

#[derive(Debug, Clone)]
pub struct FlipperHoldDirectMode {
  pub button_switch: &'static str,
  pub invert_button_switch: Option<bool>,
  pub driver_on_time: Duration,
  pub initial_pwm_power: Power,
  pub secondary_pwm_power: Power,
}

impl Default for FlipperHoldDirectMode {
  fn default() -> Self {
    Self {
      button_switch: "",
      invert_button_switch: None,
      driver_on_time: Duration::from_millis(48),
      initial_pwm_power: Power::FULL,
      secondary_pwm_power: Power::FULL,
    }
  }
}

impl DriverMode for FlipperHoldDirectMode {
  fn to_config(&self, switch_lookup: &HashMap<&'static str, usize>) -> DriverConfig {
    DriverConfig::FlipperHoldDirect {
      button_switch: switch_lookup
        .get(self.button_switch)
        .cloned()
        .expect("Flipper hold direct mode requires a valid button switch"),
      invert_button_switch: self.invert_button_switch,
      driver_on_time: self.driver_on_time,
      initial_pwm_power: self.initial_pwm_power,
      secondary_pwm_power: self.secondary_pwm_power,
    }
  }
}
