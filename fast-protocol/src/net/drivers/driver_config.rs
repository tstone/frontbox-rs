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
    off_switch: usize,
    invert_off_switch: bool,
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

/// PulseMode -- https://fastpinball.com/fast-serial-protocol/net/driver-mode/10/
#[derive(Debug, Clone)]
pub struct PulseMode {
  pub switch: Option<usize>,
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
      initial_pwm_length: Duration::from_millis(30),
      initial_pwm_power: Power::percent(100),
      secondary_pwm_length: Duration::ZERO,
      secondary_pwm_power: Power::percent(0),
      rest: Duration::from_millis(500),
    }
  }
}

impl From<PulseMode> for DriverConfig {
  fn from(c: PulseMode) -> Self {
    DriverConfig::Pulse {
      switch: c.switch,
      invert_switch: c.invert_switch,
      initial_pwm_length: c.initial_pwm_length,
      initial_pwm_power: c.initial_pwm_power,
      secondary_pwm_length: c.secondary_pwm_length,
      secondary_pwm_power: c.secondary_pwm_power,
      rest: c.rest,
    }
  }
}

/// PulseKickMode -- https://fastpinball.com/fast-serial-protocol/net/driver-mode/12/
pub struct PulseKickMode {
  pub switch: Option<usize>,
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
      initial_pwm_power: Power::percent(100),
      secondary_pwm_length: Duration::ZERO,
      secondary_pwm_power: Power::percent(0),
      kick_length: Duration::from_millis(500),
    }
  }
}

impl From<PulseKickMode> for DriverConfig {
  fn from(c: PulseKickMode) -> Self {
    DriverConfig::PulseKick {
      switch: c.switch,
      invert_switch: c.invert_switch,
      initial_pwm_length: c.initial_pwm_length,
      initial_pwm_power: c.initial_pwm_power,
      secondary_pwm_length: c.secondary_pwm_length,
      secondary_pwm_power: c.secondary_pwm_power,
      kick_length: c.kick_length,
    }
  }
}

/// PulseHoldMode -- https://fastpinball.com/fast-serial-protocol/net/driver-mode/18/
#[derive(Debug, Clone)]
pub struct PulseHoldMode {
  pub switch: Option<usize>,
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
      initial_pwm_power: Power::percent(100),
      secondary_pwm_power: Power::percent(10),
      rest: Duration::ZERO,
    }
  }
}

impl From<PulseHoldMode> for DriverConfig {
  fn from(c: PulseHoldMode) -> Self {
    DriverConfig::PulseHold {
      switch: c.switch,
      invert_switch: c.invert_switch,
      initial_pwm_length: c.initial_pwm_length,
      initial_pwm_power: c.initial_pwm_power,
      secondary_pwm_power: c.secondary_pwm_power,
      rest: c.rest,
    }
  }
}

/// PulseHoldCancelMode -- https://fastpinball.com/fast-serial-protocol/net/driver-mode/20/
#[derive(Debug, Clone)]
pub struct PulseHoldCancelMode {
  pub switch: Option<usize>,
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

impl From<PulseHoldCancelMode> for DriverConfig {
  fn from(c: PulseHoldCancelMode) -> Self {
    DriverConfig::PulseHoldCancel {
      switch: c.switch,
      invert_switch: c.invert_switch,
      off_switch: c.off_switch,
      invert_off_switch: c.invert_off_switch,
      initial_pwm_length: c.initial_pwm_length,
      secondary_pwm_power: c.secondary_pwm_power,
      secondary_pwm_length: c.secondary_pwm_length,
      rest: c.rest,
    }
  }
}

/// LongPulseMode -- https://fastpinball.com/fast-serial-protocol/net/driver-mode/70/
#[derive(Debug, Clone)]
pub struct LongPulseMode {
  pub switch: Option<usize>,
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
      initial_pwm_power: Power::percent(100),
      secondary_pwm_length: Duration::from_millis(1000),
      secondary_pwm_power: Power::percent(25),
      rest: Duration::from_millis(1000),
    }
  }
}

impl From<LongPulseMode> for DriverConfig {
  fn from(c: LongPulseMode) -> Self {
    DriverConfig::LongPulse {
      switch: c.switch,
      invert_switch: c.invert_switch,
      initial_pwm_length: c.initial_pwm_length,
      initial_pwm_power: c.initial_pwm_power,
      secondary_pwm_length: c.secondary_pwm_length,
      secondary_pwm_power: c.secondary_pwm_power,
      rest: c.rest,
    }
  }
}

#[derive(Debug, Clone)]
pub struct FlipperMainDirectMode {
  pub button_switch: usize,
  pub invert_button_switch: Option<bool>,
  pub eos_switch: usize,
  pub initial_pwm_power: Power,
  pub secondary_pwm_power: Power,
  pub max_eos_time: Duration,
  pub next_flip_refresh: Duration,
}

impl Default for FlipperMainDirectMode {
  fn default() -> Self {
    Self {
      button_switch: 0,
      invert_button_switch: None,
      eos_switch: 0,
      initial_pwm_power: Power::percent(45),
      secondary_pwm_power: Power::full(),
      max_eos_time: Duration::from_millis(60),
      next_flip_refresh: Duration::from_millis(8),
    }
  }
}

impl From<FlipperMainDirectMode> for DriverConfig {
  fn from(c: FlipperMainDirectMode) -> Self {
    DriverConfig::FlipperMainDirect {
      button_switch: c.button_switch,
      invert_button_switch: c.invert_button_switch,
      eos_switch: c.eos_switch,
      initial_pwm_power: c.initial_pwm_power,
      secondary_pwm_power: c.secondary_pwm_power,
      max_eos_time: c.max_eos_time,
      next_flip_refresh: c.next_flip_refresh,
    }
  }
}

#[derive(Debug, Clone)]
pub struct FlipperHoldDirectMode {
  pub button_switch: usize,
  pub invert_button_switch: Option<bool>,
  pub driver_on_time: Duration,
  pub initial_pwm_power: Power,
  pub secondary_pwm_power: Power,
}

impl Default for FlipperHoldDirectMode {
  fn default() -> Self {
    Self {
      button_switch: 0,
      invert_button_switch: None,
      driver_on_time: Duration::from_millis(48),
      initial_pwm_power: Power::full(),
      secondary_pwm_power: Power::full(),
    }
  }
}

impl From<FlipperHoldDirectMode> for DriverConfig {
  fn from(c: FlipperHoldDirectMode) -> Self {
    DriverConfig::FlipperHoldDirect {
      button_switch: c.button_switch,
      invert_button_switch: c.invert_button_switch,
      driver_on_time: c.driver_on_time,
      initial_pwm_power: c.initial_pwm_power,
      secondary_pwm_power: c.secondary_pwm_power,
    }
  }
}
