use std::time::Duration;

use fast_protocol::{DriverConfig, Power};

use crate::hardware::io::driver_switches::*;
use crate::operator_config::{GeneralizedConfigValue, HardwareValue};
use crate::prelude::*;

/// Mode 20 - Pulse then indefinitely hold the driver on until the trigger (flip) is deactivated -OR- the cancel
/// switch (flop) is activated.
/// <https://fastpinball.com/fast-serial-protocol/net/driver-mode/20/>
#[derive(Clone, Debug)]
pub struct PulseHoldCancelMode {
  /// What causes the driver to fire (be triggered)
  pub trigger_mode: DriverTriggerDualMode,
  pub initial_pwm_length: HardwareValue<Duration>,
  pub initial_pwm_power: HardwareValue<Power>,
  pub secondary_pwm_power: HardwareValue<Power>,
  /// Time after the driver goes off before it can be triggered again
  pub rest: HardwareValue<Duration>,
}

impl Default for PulseHoldCancelMode {
  fn default() -> Self {
    Self {
      trigger_mode: DriverTriggerDualMode::Disabled,
      initial_pwm_length: HardwareValue::Fixed(Duration::from_millis(30)),
      initial_pwm_power: HardwareValue::Fixed(Power::FULL),
      secondary_pwm_power: HardwareValue::Fixed(Power::EIGHTH),
      rest: HardwareValue::Fixed(Duration::from_millis(255)),
    }
  }
}

impl PulseHoldCancelMode {
  pub(super) fn to_config(&self, ctx: &BootSnapshot) -> DriverConfig {
    let (flip_switch, invert_flip_switch, flop_switch, invert_flop_switch) =
      get_switch_ids_and_inverts(&self.trigger_mode, ctx);

    DriverConfig::PulseHoldCancel {
      switch: flip_switch,
      invert_switch: invert_flip_switch,
      off_switch: flop_switch,
      invert_off_switch: invert_flop_switch,
      initial_max_on_time: self.initial_pwm_length.resolve(&ctx.operator_config),
      initial_pwm_power: self.initial_pwm_power.resolve(&ctx.operator_config),
      secondary_pwm_power: self.secondary_pwm_power.resolve(&ctx.operator_config),
      rest: self.rest.resolve(&ctx.operator_config),
    }
  }

  pub(super) fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    [
      self.initial_pwm_length.generalized_config_value(),
      self.initial_pwm_power.generalized_config_value(),
      self.secondary_pwm_power.generalized_config_value(),
      self.rest.generalized_config_value(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}

#[derive(Clone, Debug, Default)]
pub struct PulseHoldCancelModeBuilder {
  mode: PulseHoldCancelMode,
}

impl PulseHoldCancelModeBuilder {
  /// What causes the driver to fire (be triggered)
  pub fn trigger_mode(mut self, trigger_mode: DriverTriggerDualMode) -> Self {
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
    DriverMode::PulseHoldCancel(self.mode)
  }
}
