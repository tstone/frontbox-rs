use std::time::Duration;

use fast_protocol::{DriverConfig, Power};

use crate::hardware::io::driver_switches::*;
use crate::operator_config::{GeneralizedConfigValue, HardwareValue};
use crate::prelude::*;

/// Mode 30 - Insert a delay between when the switch is triggered and the driver fires.
/// Useful for things kickbacks where a bit of delay needs to be added into the automatic flow.
/// <https://fastpinball.com/fast-serial-protocol/net/driver-mode/30/>
#[derive(Clone, Debug)]
pub struct DelayedPulseMode {
  /// What causes the driver to fire (be triggered)
  pub trigger_mode: DriverTriggerMode,
  pub delay_length: HardwareValue<Duration>,
  pub initial_full_power_length: HardwareValue<Duration>,
  pub secondary_pwm_length: HardwareValue<Duration>,
  pub secondary_pwm_power: HardwareValue<Power>,
  /// Time after the driver goes off before it can be triggered again
  pub rest: HardwareValue<Duration>,
}

impl Default for DelayedPulseMode {
  fn default() -> Self {
    Self {
      trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
      delay_length: HardwareValue::Fixed(Duration::from_millis(30)),
      initial_full_power_length: HardwareValue::Fixed(Duration::from_millis(30)),
      secondary_pwm_length: HardwareValue::Fixed(Duration::ZERO),
      secondary_pwm_power: HardwareValue::Fixed(Power::ZERO),
      rest: HardwareValue::Fixed(Duration::from_millis(80)),
    }
  }
}

impl DelayedPulseMode {
  pub(super) fn to_config(&self, ctx: &BootSnapshot) -> DriverConfig {
    let (switch, invert_switch) = get_switch_id_and_invert(&self.trigger_mode, ctx);

    DriverConfig::DelayedPulse {
      switch,
      invert_switch,
      delay_length: self.delay_length.resolve(&ctx.operator_config),
      initial_full_power_length: self.initial_full_power_length.resolve(&ctx.operator_config),
      secondary_pwm_length: self.secondary_pwm_length.resolve(&ctx.operator_config),
      secondary_pwm_power: self.secondary_pwm_power.resolve(&ctx.operator_config),
      rest: self.rest.resolve(&ctx.operator_config),
    }
  }

  pub(super) fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    [
      self.delay_length.generalized_config_value(),
      self.initial_full_power_length.generalized_config_value(),
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
pub struct DelayedPulseModeBuilder {
  mode: DelayedPulseMode,
}

impl DelayedPulseModeBuilder {
  /// What causes the driver to fire (be triggered)
  pub fn trigger_mode(mut self, trigger_mode: DriverTriggerMode) -> Self {
    self.mode.trigger_mode = trigger_mode;
    self
  }

  pub fn delay_length(mut self, value: impl Into<HardwareValue<Duration>>) -> Self {
    self.mode.delay_length = value.into();
    self
  }

  pub fn initial_full_power_length(mut self, value: impl Into<HardwareValue<Duration>>) -> Self {
    self.mode.initial_full_power_length = value.into();
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
    DriverMode::DelayedPulse(self.mode)
  }
}
