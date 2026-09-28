use std::time::Duration;

use fast_protocol::{DriverConfig, Power};

use crate::hardware::io::driver_switches::*;
use crate::operator_config::{GeneralizedConfigValue, HardwareValue};
use crate::prelude::*;

/// Mode 75 - Pulse the driver until the trigger (flip) is deactivated -OR- the cancel switch (flop) is activated.
/// <https://fastpinball.com/fast-serial-protocol/net/driver-mode/75/>
#[derive(Clone, Debug)]
pub struct PulseCancelMode {
  /// What causes the driver to fire (be triggered)
  pub trigger_mode: DriverTriggerDualMode,
  pub initial_full_power_length: HardwareValue<Duration>,
  pub secondary_power_length: HardwareValue<Duration>,
  pub secondary_pwm_power: HardwareValue<Power>,
  /// Time after the driver goes off before it can be triggered again
  pub rest: HardwareValue<Duration>,
}

impl Default for PulseCancelMode {
  fn default() -> Self {
    Self {
      trigger_mode: DriverTriggerDualMode::Disabled,
      initial_full_power_length: HardwareValue::Fixed(Duration::from_millis(30)),
      secondary_power_length: HardwareValue::Fixed(Duration::from_millis(500)),
      secondary_pwm_power: HardwareValue::Fixed(Power::EIGHTH),
      rest: HardwareValue::Fixed(Duration::from_millis(255)),
    }
  }
}

impl PulseCancelMode {
  pub(super) fn to_config(&self, ctx: &BootSnapshot) -> DriverConfig {
    let (flip_switch, invert_flip_switch, flop_switch, invert_flop_switch) =
      get_switch_ids_and_inverts(&self.trigger_mode, ctx);

    DriverConfig::PulseCancel {
      switch: flip_switch,
      invert_switch: invert_flip_switch,
      off_switch: flop_switch,
      invert_off_switch: invert_flop_switch,
      initial_full_power_length: self.initial_full_power_length.resolve(&ctx.operator_config),
      secondary_power_length: self.secondary_power_length.resolve(&ctx.operator_config),
      secondary_pwm_power: self.secondary_pwm_power.resolve(&ctx.operator_config),
      rest: self.rest.resolve(&ctx.operator_config),
    }
  }

  pub(super) fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    [
      self.initial_full_power_length.generalized_config_value(),
      self.secondary_power_length.generalized_config_value(),
      self.secondary_pwm_power.generalized_config_value(),
      self.rest.generalized_config_value(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}

#[derive(Clone, Debug, Default)]
pub struct PulseCancelModeBuilder {
  mode: PulseCancelMode,
}

impl PulseCancelModeBuilder {
  /// What causes the driver to fire (be triggered)
  pub fn trigger_mode(mut self, trigger_mode: DriverTriggerDualMode) -> Self {
    self.mode.trigger_mode = trigger_mode;
    self
  }

  pub fn initial_full_power_length(mut self, value: impl Into<HardwareValue<Duration>>) -> Self {
    self.mode.initial_full_power_length = value.into();
    self
  }

  pub fn secondary_power_length(mut self, value: impl Into<HardwareValue<Duration>>) -> Self {
    self.mode.secondary_power_length = value.into();
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
    DriverMode::PulseCancel(self.mode)
  }
}
