use std::time::Duration;

use crate::Power;

#[derive(Debug, Clone)]
pub enum DriverConfig {
  Disabled,
  Pulse {
    switch: Option<usize>,
    invert_switch: Option<bool>,
    initial_pwm_length: Duration,
    initial_pwm_power: Power,
    secondary_pwm_length: Duration,
    secondary_pwm_power: Power,
    rest: Duration,
  },
  PulseKick {
    switch: Option<usize>,
    invert_switch: Option<bool>,
    initial_pwm_length: Duration,
    initial_pwm_power: Power,
    secondary_pwm_length: Duration,
    secondary_pwm_power: Power,
    kick_length: Duration,
  },
  PulseHold {
    switch: Option<usize>,
    invert_switch: Option<bool>,
    initial_pwm_length: Duration,
    initial_pwm_power: Power,
    secondary_pwm_power: Power,
    rest: Duration,
  },
  PulseHoldCancel {
    switch: Option<usize>,
    invert_switch: Option<bool>,
    off_switch: Option<usize>,
    invert_off_switch: Option<bool>,
    initial_pwm_length: Duration,
    secondary_pwm_power: Power,
    secondary_pwm_length: Duration,
    rest: Duration,
  },
  LongPulse {
    switch: Option<usize>,
    invert_switch: Option<bool>,
    initial_pwm_length: Duration,
    initial_pwm_power: Power,
    secondary_pwm_length: Duration,
    secondary_pwm_power: Power,
    rest: Duration,
  },
  FlipperMainDirect {
    button_switch: usize,
    invert_button_switch: Option<bool>,
    eos_switch: usize,
    initial_pwm_power: Power,
    secondary_pwm_power: Power,
    max_eos_time: Duration,
    next_flip_refresh: Duration,
  },
  FlipperHoldDirect {
    button_switch: usize,
    invert_button_switch: Option<bool>,
    driver_on_time: Duration,
    initial_pwm_power: Power,
    secondary_pwm_power: Power,
  },
}

impl DriverConfig {
  pub fn switch_id(&self) -> Option<usize> {
    match self {
      DriverConfig::Disabled => None,
      DriverConfig::Pulse { switch, .. } => *switch,
      DriverConfig::PulseKick { switch, .. } => *switch,
      DriverConfig::PulseHold { switch, .. } => *switch,
      DriverConfig::PulseHoldCancel { switch, .. } => *switch,
      DriverConfig::LongPulse { switch, .. } => *switch,
      DriverConfig::FlipperMainDirect { eos_switch, .. } => Some(*eos_switch),
      DriverConfig::FlipperHoldDirect { button_switch, .. } => Some(*button_switch),
    }
  }
}
