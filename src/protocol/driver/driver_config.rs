use std::time::Duration;

use crate::protocol::prelude::Power;

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
    invert_office_switch: bool,
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
    button_switch: Option<usize>,
    invert_button_switch: Option<bool>,
    eos_switch: usize,
    initial_pwm_power: Power,
    secondary_pwm_power: Power,
    max_eos_time: Duration,
    next_flip_refresh: Duration,
  },
  FlipperHoldDirect {
    button_switch: Option<usize>,
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
      DriverConfig::PulseHold { switch, .. } => *switch,
      DriverConfig::PulseHoldCancel { switch, .. } => *switch,
      DriverConfig::LongPulse { switch, .. } => *switch,
      DriverConfig::FlipperMainDirect { eos_switch, .. } => Some(*eos_switch),
      DriverConfig::FlipperHoldDirect { button_switch, .. } => *button_switch,
    }
  }

  pub fn pulse() -> PulseBuilder {
    PulseBuilder::default()
  }

  pub fn pulse_hold() -> PulseHoldBuilder {
    PulseHoldBuilder::default()
  }

  pub fn pulse_hold_cancel() -> PulseHoldCancelBuilder {
    PulseHoldCancelBuilder::default()
  }

  pub fn long_pulse() -> LongPulseBuilder {
    LongPulseBuilder::default()
  }

  pub fn flipper_main_direct() -> FlipperMainDirectBuilder {
    FlipperMainDirectBuilder::default()
  }

  pub fn flipper_hold_direct() -> FlipperHoldDirectBuilder {
    FlipperHoldDirectBuilder::default()
  }
}

// Builder for Pulse variant
pub struct PulseBuilder {
  switch: Option<usize>,
  invert_switch: Option<bool>,
  initial_pwm_length: Option<Duration>,
  initial_pwm_power: Option<Power>,
  secondary_pwm_length: Option<Duration>,
  secondary_pwm_power: Option<Power>,
  rest: Option<Duration>,
}

impl Default for PulseBuilder {
  fn default() -> Self {
    Self {
      switch: None,
      invert_switch: None,
      initial_pwm_length: Some(Duration::from_millis(20)),
      initial_pwm_power: Some(Power::full()),
      secondary_pwm_length: None,
      secondary_pwm_power: None,
      rest: None,
    }
  }
}

impl PulseBuilder {
  pub fn switch(mut self, switch: usize) -> Self {
    self.switch = Some(switch);
    self
  }

  pub fn invert_switch(mut self, invert: bool) -> Self {
    self.invert_switch = Some(invert);
    self
  }

  pub fn initial_pwm_length(mut self, length: Duration) -> Self {
    self.initial_pwm_length = Some(length);
    self
  }

  pub fn initial_pwm_power(mut self, power: Power) -> Self {
    self.initial_pwm_power = Some(power);
    self
  }

  pub fn secondary_pwm_length(mut self, length: Duration) -> Self {
    self.secondary_pwm_length = Some(length);
    self
  }

  pub fn secondary_pwm_power(mut self, power: Power) -> Self {
    self.secondary_pwm_power = Some(power);
    self
  }

  pub fn rest(mut self, rest: Duration) -> Self {
    self.rest = Some(rest);
    self
  }

  pub fn build(self) -> DriverConfig {
    DriverConfig::Pulse {
      switch: self.switch,
      invert_switch: self.invert_switch,
      initial_pwm_length: self.initial_pwm_length.unwrap_or(Duration::from_millis(10)),
      initial_pwm_power: self.initial_pwm_power.unwrap_or(Power::full()),
      secondary_pwm_length: self
        .secondary_pwm_length
        .unwrap_or(Duration::from_millis(0)),
      secondary_pwm_power: self.secondary_pwm_power.unwrap_or(Power::off()),
      rest: self.rest.unwrap_or(Duration::from_millis(0)),
    }
  }
}

// Builder for PulseHold variant
#[derive(Default)]
pub struct PulseHoldBuilder {
  switch: Option<usize>,
  invert_switch: Option<bool>,
  initial_pwm_length: Option<Duration>,
  initial_pwm_power: Option<Power>,
  secondary_pwm_power: Option<Power>,
  rest: Option<Duration>,
}

impl PulseHoldBuilder {
  pub fn switch(mut self, switch: usize) -> Self {
    self.switch = Some(switch);
    self
  }

  pub fn invert_switch(mut self, invert: bool) -> Self {
    self.invert_switch = Some(invert);
    self
  }

  pub fn initial_pwm_length(mut self, length: Duration) -> Self {
    self.initial_pwm_length = Some(length);
    self
  }

  pub fn initial_pwm_power(mut self, power: Power) -> Self {
    self.initial_pwm_power = Some(power);
    self
  }

  pub fn secondary_pwm_power(mut self, power: Power) -> Self {
    self.secondary_pwm_power = Some(power);
    self
  }

  pub fn rest(mut self, rest: Duration) -> Self {
    self.rest = Some(rest);
    self
  }

  pub fn build(self) -> DriverConfig {
    DriverConfig::PulseHold {
      switch: self.switch,
      invert_switch: self.invert_switch,
      initial_pwm_length: self.initial_pwm_length.unwrap_or(Duration::from_millis(10)),
      initial_pwm_power: self.initial_pwm_power.unwrap_or(Power::full()),
      secondary_pwm_power: self.secondary_pwm_power.unwrap_or(Power::off()),
      rest: self.rest.unwrap_or(Duration::from_millis(0)),
    }
  }
}

// Builder for PulseHoldCancel variant
#[derive(Default)]
pub struct PulseHoldCancelBuilder {
  switch: Option<usize>,
  invert_switch: Option<bool>,
  off_switch: Option<usize>,
  invert_office_switch: Option<bool>,
  initial_pwm_length: Option<Duration>,
  secondary_pwm_power: Option<Power>,
  secondary_pwm_length: Option<Duration>,
  rest: Option<Duration>,
}

impl PulseHoldCancelBuilder {
  pub fn switch(mut self, switch: usize) -> Self {
    self.switch = Some(switch);
    self
  }

  pub fn invert_switch(mut self, invert: bool) -> Self {
    self.invert_switch = Some(invert);
    self
  }

  pub fn off_switch(mut self, switch: usize) -> Self {
    self.off_switch = Some(switch);
    self
  }

  pub fn invert_office_switch(mut self, invert: bool) -> Self {
    self.invert_office_switch = Some(invert);
    self
  }

  pub fn initial_pwm_length(mut self, length: Duration) -> Self {
    self.initial_pwm_length = Some(length);
    self
  }

  pub fn secondary_pwm_power(mut self, power: Power) -> Self {
    self.secondary_pwm_power = Some(power);
    self
  }

  pub fn secondary_pwm_length(mut self, length: Duration) -> Self {
    self.secondary_pwm_length = Some(length);
    self
  }

  pub fn rest(mut self, rest: Duration) -> Self {
    self.rest = Some(rest);
    self
  }

  pub fn build(self) -> DriverConfig {
    DriverConfig::PulseHoldCancel {
      switch: self.switch,
      invert_switch: self.invert_switch,
      off_switch: self
        .off_switch
        .expect("off_switch is required for PulseHoldCancel"),
      invert_office_switch: self.invert_office_switch.unwrap_or(false),
      initial_pwm_length: self.initial_pwm_length.unwrap_or(Duration::from_millis(10)),
      secondary_pwm_power: self.secondary_pwm_power.unwrap_or(Power::off()),
      secondary_pwm_length: self
        .secondary_pwm_length
        .unwrap_or(Duration::from_millis(0)),
      rest: self.rest.unwrap_or(Duration::from_millis(0)),
    }
  }
}

// Builder for LongPulse variant
#[derive(Default)]
pub struct LongPulseBuilder {
  switch: Option<usize>,
  invert_switch: Option<bool>,
  initial_pwm_length: Option<Duration>,
  initial_pwm_power: Option<Power>,
  secondary_pwm_length: Option<Duration>,
  secondary_pwm_power: Option<Power>,
  rest: Option<Duration>,
}

impl LongPulseBuilder {
  pub fn switch(mut self, switch: usize) -> Self {
    self.switch = Some(switch);
    self
  }

  pub fn invert_switch(mut self, invert: bool) -> Self {
    self.invert_switch = Some(invert);
    self
  }

  pub fn initial_pwm_length(mut self, length: Duration) -> Self {
    self.initial_pwm_length = Some(length);
    self
  }

  pub fn initial_pwm_power(mut self, power: Power) -> Self {
    self.initial_pwm_power = Some(power);
    self
  }

  pub fn secondary_pwm_length(mut self, length: Duration) -> Self {
    self.secondary_pwm_length = Some(length);
    self
  }

  pub fn secondary_pwm_power(mut self, power: Power) -> Self {
    self.secondary_pwm_power = Some(power);
    self
  }

  pub fn rest(mut self, rest: Duration) -> Self {
    self.rest = Some(rest);
    self
  }

  pub fn build(self) -> DriverConfig {
    DriverConfig::LongPulse {
      switch: self.switch,
      invert_switch: self.invert_switch,
      initial_pwm_length: self.initial_pwm_length.unwrap_or(Duration::from_millis(10)),
      initial_pwm_power: self.initial_pwm_power.unwrap_or(Power::full()),
      secondary_pwm_length: self
        .secondary_pwm_length
        .unwrap_or(Duration::from_millis(0)),
      secondary_pwm_power: self.secondary_pwm_power.unwrap_or(Power::off()),
      rest: self.rest.unwrap_or(Duration::from_millis(0)),
    }
  }
}

// Builder for FlipperMainDirect variant
#[derive(Default)]
pub struct FlipperMainDirectBuilder {
  button_switch: Option<usize>,
  invert_button_switch: Option<bool>,
  eos_switch: Option<usize>,
  initial_pwm_power: Option<Power>,
  secondary_pwm_power: Option<Power>,
  max_eos_time: Option<Duration>,
  next_flip_refresh: Option<Duration>,
}

impl FlipperMainDirectBuilder {
  pub fn button_switch(mut self, switch: usize) -> Self {
    self.button_switch = Some(switch);
    self
  }

  pub fn invert_button_switch(mut self, invert: bool) -> Self {
    self.invert_button_switch = Some(invert);
    self
  }

  pub fn eos_switch(mut self, switch: usize) -> Self {
    self.eos_switch = Some(switch);
    self
  }

  pub fn initial_pwm_power(mut self, power: Power) -> Self {
    self.initial_pwm_power = Some(power);
    self
  }

  pub fn secondary_pwm_power(mut self, power: Power) -> Self {
    self.secondary_pwm_power = Some(power);
    self
  }

  pub fn max_eos_time(mut self, time: Duration) -> Self {
    self.max_eos_time = Some(time);
    self
  }

  pub fn next_flip_refresh(mut self, time: Duration) -> Self {
    self.next_flip_refresh = Some(time);
    self
  }

  pub fn build(self) -> DriverConfig {
    DriverConfig::FlipperMainDirect {
      button_switch: self.button_switch,
      invert_button_switch: self.invert_button_switch,
      eos_switch: self
        .eos_switch
        .expect("eos_switch is required for FlipperMainDirect"),
      initial_pwm_power: self.initial_pwm_power.unwrap_or(Power::full()),
      secondary_pwm_power: self.secondary_pwm_power.unwrap_or(Power::off()),
      max_eos_time: self.max_eos_time.unwrap_or(Duration::from_millis(100)),
      next_flip_refresh: self.next_flip_refresh.unwrap_or(Duration::from_millis(100)),
    }
  }
}

// Builder for FlipperHoldDirect variant
#[derive(Default)]
pub struct FlipperHoldDirectBuilder {
  button_switch: Option<usize>,
  invert_button_switch: Option<bool>,
  driver_on_time: Option<Duration>,
  initial_pwm_power: Option<Power>,
  secondary_pwm_power: Option<Power>,
}

impl FlipperHoldDirectBuilder {
  pub fn button_switch(mut self, switch: usize) -> Self {
    self.button_switch = Some(switch);
    self
  }

  pub fn invert_button_switch(mut self, invert: bool) -> Self {
    self.invert_button_switch = Some(invert);
    self
  }

  pub fn driver_on_time(mut self, time: Duration) -> Self {
    self.driver_on_time = Some(time);
    self
  }

  pub fn initial_pwm_power(mut self, power: Power) -> Self {
    self.initial_pwm_power = Some(power);
    self
  }

  pub fn secondary_pwm_power(mut self, power: Power) -> Self {
    self.secondary_pwm_power = Some(power);
    self
  }

  pub fn build(self) -> DriverConfig {
    DriverConfig::FlipperHoldDirect {
      button_switch: self.button_switch,
      invert_button_switch: self.invert_button_switch,
      driver_on_time: self.driver_on_time.unwrap_or(Duration::from_millis(10)),
      initial_pwm_power: self.initial_pwm_power.unwrap_or(Power::full()),
      secondary_pwm_power: self.secondary_pwm_power.unwrap_or(Power::off()),
    }
  }
}
