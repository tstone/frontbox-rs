use std::time::Duration;

use fast_protocol::{DriverConfig, Power};

use crate::hardware::io::driver_switches::*;
use crate::operator_config::{GeneralizedConfigValue, HardwareValue};
use crate::prelude::*;

/// Mode 12 - Sends up to 2 variable PWM times, then kicks (full power) at the end of the cycle. Useful for gently
/// moving a coil and then kicking it the rest of the way, e.g. VUK or trough eject. Reduces force applied
/// to ball by ensuring a plunger has full contact with the ball before a full kick occurs.
/// <https://fastpinball.com/fast-serial-protocol/net/driver-mode/12/>
#[derive(Clone, Debug, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PulseKickMode {
  /// What causes the driver to fire (be triggered)
  pub trigger_mode: DriverTriggerMode,
  pub initial_pwm_length: HardwareValue<Duration>,
  pub initial_pwm_power: HardwareValue<Power>,
  pub secondary_pwm_length: HardwareValue<Duration>,
  pub secondary_pwm_power: HardwareValue<Power>,
  /// Time that the driver is held at full power after the initial and secondary PWM times
  pub kick_length: HardwareValue<Duration>,
}

impl Default for PulseKickMode {
  fn default() -> Self {
    Self {
      trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
      initial_pwm_length: HardwareValue::Fixed(Duration::from_millis(30)),
      initial_pwm_power: HardwareValue::Fixed(Power::FULL),
      secondary_pwm_length: HardwareValue::Fixed(Duration::ZERO),
      secondary_pwm_power: HardwareValue::Fixed(Power::ZERO),
      kick_length: HardwareValue::Fixed(Duration::from_millis(500)),
    }
  }
}

impl PulseKickMode {
  pub(super) fn to_config(&self, ctx: &BootSnapshot) -> DriverConfig {
    let (switch, invert_switch) = get_switch_id_and_invert(&self.trigger_mode, ctx);

    DriverConfig::PulseKick {
      switch,
      invert_switch,
      initial_pwm_length: self.initial_pwm_length.resolve(&ctx.operator_config),
      initial_pwm_power: self.initial_pwm_power.resolve(&ctx.operator_config),
      secondary_pwm_length: self.secondary_pwm_length.resolve(&ctx.operator_config),
      secondary_pwm_power: self.secondary_pwm_power.resolve(&ctx.operator_config),
      kick_length: self.kick_length.resolve(&ctx.operator_config),
    }
  }

  pub(super) fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    [
      self.initial_pwm_length.generalized_config_value(),
      self.initial_pwm_power.generalized_config_value(),
      self.secondary_pwm_length.generalized_config_value(),
      self.secondary_pwm_power.generalized_config_value(),
      self.kick_length.generalized_config_value(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}

#[derive(Clone, Debug, Default)]
pub struct PulseKickModeBuilder {
  mode: PulseKickMode,
}

impl PulseKickModeBuilder {
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

  /// Time that the driver is held at full power after the initial and secondary PWM times
  pub fn kick_length(mut self, value: impl Into<HardwareValue<Duration>>) -> Self {
    self.mode.kick_length = value.into();
    self
  }

  pub fn build(self) -> DriverMode {
    DriverMode::PulseKick(self.mode)
  }
}
