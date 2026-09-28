use std::time::Duration;

use fast_protocol::{DriverConfig, Power};

use crate::hardware::io::driver_switches::*;
use crate::operator_config::{GeneralizedConfigValue, HardwareValue};
use crate::prelude::*;

/// Mode 70 - Pulse the driver for an initial time (up to 255ms), then hold it for a secondary time (up to 25s).
/// <https://fastpinball.com/fast-serial-protocol/net/driver-mode/70/>
#[derive(Clone, Debug)]
pub struct LongPulseMode {
  /// What causes the driver to fire (be triggered)
  pub trigger_mode: DriverTriggerMode,
  pub initial_pwm_length: HardwareValue<Duration>,
  pub initial_pwm_power: HardwareValue<Power>,
  pub secondary_pwm_length: HardwareValue<Duration>,
  pub secondary_pwm_power: HardwareValue<Power>,
  /// Time after the driver goes off before it can be triggered again
  pub rest: HardwareValue<Duration>,
}

impl Default for LongPulseMode {
  fn default() -> Self {
    Self {
      trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
      initial_pwm_length: HardwareValue::Fixed(Duration::from_millis(200)),
      initial_pwm_power: HardwareValue::Fixed(Power::FULL),
      secondary_pwm_length: HardwareValue::Fixed(Duration::from_millis(1000)),
      secondary_pwm_power: HardwareValue::Fixed(Power::QUARTER),
      rest: HardwareValue::Fixed(Duration::from_millis(255)),
    }
  }
}

impl LongPulseMode {
  pub(super) fn to_config(&self, ctx: &BootSnapshot) -> DriverConfig {
    let (switch, invert_switch) = get_switch_id_and_invert(&self.trigger_mode, ctx);

    DriverConfig::LongPulse {
      switch,
      invert_switch,
      initial_pwm_length: self.initial_pwm_length.resolve(&ctx.operator_config),
      initial_pwm_power: self.initial_pwm_power.resolve(&ctx.operator_config),
      secondary_pwm_length: self.secondary_pwm_length.resolve(&ctx.operator_config),
      secondary_pwm_power: self.secondary_pwm_power.resolve(&ctx.operator_config),
      rest: self.rest.resolve(&ctx.operator_config),
    }
  }

  pub(super) fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    [
      self.initial_pwm_length.generalized_config_value(),
      self.initial_pwm_power.generalized_config_value(),
      self.secondary_pwm_length.generalized_config_value(),
      self.secondary_pwm_power.generalized_config_value(),
      self.rest.generalized_config_value(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}

#[derive(Clone, Debug, Default)]
pub struct LongPulseModeBuilder {
  mode: LongPulseMode,
}

impl LongPulseModeBuilder {
  /// What causes the driver to fire (be triggered)
  pub fn trigger_mode(mut self, trigger_mode: DriverTriggerMode) -> Self {
    self.mode.trigger_mode = trigger_mode;
    self
  }

  pub fn initial_pwm_length(mut self, value: impl Into<HardwareValue<Duration>>) -> Self {
    self.mode.initial_pwm_length = value.into();
    self
  }

  pub fn initial_pwm_power(mut self, value: impl Into<HardwareValue<Power>>) -> Self {
    self.mode.initial_pwm_power = value.into();
    self
  }

  pub fn secondary_pwm_length(mut self, value: impl Into<HardwareValue<Duration>>) -> Self {
    self.mode.secondary_pwm_length = value.into();
    self
  }

  pub fn secondary_pwm_power(mut self, value: impl Into<HardwareValue<Power>>) -> Self {
    self.mode.secondary_pwm_power = value.into();
    self
  }

  /// Time after the driver goes off before it can be triggered again
  pub fn rest(mut self, value: impl Into<HardwareValue<Duration>>) -> Self {
    self.mode.rest = value.into();
    self
  }

  pub fn build(self) -> DriverMode {
    DriverMode::LongPulse(self.mode)
  }
}
