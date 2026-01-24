use std::time::Duration;

use crate::Switch;
use crate::hardware::power::Power;

pub enum DriverConfig {
  Disabled,
  Pulse {
    switch: Option<Switch>,
    invert_switch: Option<bool>,
    initial_pwm_length: Duration,
    initial_pwm_power: Power,
    secondary_pwm_length: Duration,
    secondary_pwm_power: Power,
    rest: Duration,
  },
  PulseHold {
    switch: Option<Switch>,
    invert_switch: Option<bool>,
    initial_pwm_length: Duration,
    initial_pwm_power: Power,
    secondary_pwm_power: Power,
    rest: Duration,
  },
  PulseHoldCancel {
    switch: Option<Switch>,
    invert_switch: Option<bool>,
    off_switch: Switch,
    invert_office_switch: bool,
    initial_pwm_length: Duration,
    secondary_pwm_power: Power,
    secondary_pwm_length: Duration,
    rest: Duration,
  },
  LongPulse {
    switch: Option<Switch>,
    invert_switch: Option<bool>,
    initial_pwm_length: Duration,
    initial_pwm_power: Power,
    secondary_pwm_length: Duration,
    secondary_pwm_power: Power,
    rest: Duration,
  },
  FlipperMainDirect {
    button_switch: Option<Switch>,
    invert_button_switch: Option<bool>,
    eos_switch: Switch,
    initial_pwm_power: Power,
    secondary_pwm_power: Power,
    max_eos_time: Duration,
    next_flip_refresh: Duration,
  },
  FlipperHoldDirect {
    button_switch: Option<Switch>,
    invert_button_switch: Option<bool>,
    driver_on_time: Duration,
    initial_pwm_power: Power,
    secondary_pwm_power: Power,
  },
}

impl DriverConfig {
  /// Create a builder for a Pulse driver config
  pub fn pulse() -> PulseBuilder {
    PulseBuilder::default()
  }

  /// Create a builder for a PulseHold driver config
  pub fn pulse_hold() -> PulseHoldBuilder {
    PulseHoldBuilder::default()
  }

  /// Create a builder for a PulseHoldCancel driver config
  pub fn pulse_hold_cancel() -> PulseHoldCancelBuilder {
    PulseHoldCancelBuilder::default()
  }

  /// Create a builder for a LongPulse driver config
  pub fn long_pulse() -> LongPulseBuilder {
    LongPulseBuilder::default()
  }

  /// Create a builder for a FlipperMainDirect driver config
  pub fn flipper_main_direct() -> FlipperMainDirectBuilder {
    FlipperMainDirectBuilder::default()
  }

  /// Create a builder for a FlipperHoldDirect driver config
  pub fn flipper_hold_direct() -> FlipperHoldDirectBuilder {
    FlipperHoldDirectBuilder::default()
  }
}

// Builder for Pulse variant
#[derive(Default)]
pub struct PulseBuilder {
  switch: Option<Switch>,
  invert_switch: Option<bool>,
  initial_pwm_length: Option<Duration>,
  initial_pwm_power: Option<Power>,
  secondary_pwm_length: Option<Duration>,
  secondary_pwm_power: Option<Power>,
  rest: Option<Duration>,
}

impl PulseBuilder {
  pub fn switch(mut self, switch: Switch) -> Self {
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
  switch: Option<Switch>,
  invert_switch: Option<bool>,
  initial_pwm_length: Option<Duration>,
  initial_pwm_power: Option<Power>,
  secondary_pwm_power: Option<Power>,
  rest: Option<Duration>,
}

impl PulseHoldBuilder {
  pub fn switch(mut self, switch: Switch) -> Self {
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
  switch: Option<Switch>,
  invert_switch: Option<bool>,
  off_switch: Option<Switch>,
  invert_office_switch: Option<bool>,
  initial_pwm_length: Option<Duration>,
  secondary_pwm_power: Option<Power>,
  secondary_pwm_length: Option<Duration>,
  rest: Option<Duration>,
}

impl PulseHoldCancelBuilder {
  pub fn switch(mut self, switch: Switch) -> Self {
    self.switch = Some(switch);
    self
  }

  pub fn invert_switch(mut self, invert: bool) -> Self {
    self.invert_switch = Some(invert);
    self
  }

  pub fn off_switch(mut self, switch: Switch) -> Self {
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
  switch: Option<Switch>,
  invert_switch: Option<bool>,
  initial_pwm_length: Option<Duration>,
  initial_pwm_power: Option<Power>,
  secondary_pwm_length: Option<Duration>,
  secondary_pwm_power: Option<Power>,
  rest: Option<Duration>,
}

impl LongPulseBuilder {
  pub fn switch(mut self, switch: Switch) -> Self {
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
  button_switch: Option<Switch>,
  invert_button_switch: Option<bool>,
  eos_switch: Option<Switch>,
  initial_pwm_power: Option<Power>,
  secondary_pwm_power: Option<Power>,
  max_eos_time: Option<Duration>,
  next_flip_refresh: Option<Duration>,
}

impl FlipperMainDirectBuilder {
  pub fn button_switch(mut self, switch: Switch) -> Self {
    self.button_switch = Some(switch);
    self
  }

  pub fn invert_button_switch(mut self, invert: bool) -> Self {
    self.invert_button_switch = Some(invert);
    self
  }

  pub fn eos_switch(mut self, switch: Switch) -> Self {
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
  button_switch: Option<Switch>,
  invert_button_switch: Option<bool>,
  driver_on_time: Option<Duration>,
  initial_pwm_power: Option<Power>,
  secondary_pwm_power: Option<Power>,
}

impl FlipperHoldDirectBuilder {
  pub fn button_switch(mut self, switch: Switch) -> Self {
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
