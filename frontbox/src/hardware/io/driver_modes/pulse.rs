use std::time::Duration;

use fast_protocol::{DriverConfig, Power};

use crate::hardware::io::driver_switches::*;
use crate::operator_config::{ConfigDefinition, HardwareValue};
use crate::prelude::*;

/// Mode 10 - Pulse the driver, up to 255ms, when triggered.
/// <https://fastpinball.com/fast-serial-protocol/net/driver-mode/10/>
#[derive(Clone, Debug, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PulseMode {
  /// What causes the driver to fire (be triggered)
  pub trigger_mode: DriverTriggerMode,
  pub initial_pwm_length: HardwareValue<Duration>,
  pub initial_pwm_power: HardwareValue<Power>,
  pub secondary_pwm_length: HardwareValue<Duration>,
  pub secondary_pwm_power: HardwareValue<Power>,
  /// Time after the driver goes off before it can be triggered again
  pub rest: HardwareValue<Duration>,
}

impl Default for PulseMode {
  fn default() -> Self {
    Self {
      trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
      initial_pwm_length: HardwareValue::Fixed(Duration::from_millis(20)),
      initial_pwm_power: HardwareValue::Fixed(Power::FULL),
      secondary_pwm_length: HardwareValue::Fixed(Duration::ZERO),
      secondary_pwm_power: HardwareValue::Fixed(Power::ZERO),
      rest: HardwareValue::Fixed(Duration::from_millis(80)),
    }
  }
}

impl PulseMode {
  pub(super) fn to_config(&self, snap: &BootSnapshot) -> DriverConfig {
    let (switch, invert_switch) = get_switch_id_and_invert(&self.trigger_mode, snap);

    DriverConfig::Pulse {
      switch,
      invert_switch,
      initial_pwm_length: self.initial_pwm_length.resolve(snap),
      initial_pwm_power: self.initial_pwm_power.resolve(snap),
      secondary_pwm_length: self.secondary_pwm_length.resolve(snap),
      secondary_pwm_power: self.secondary_pwm_power.resolve(snap),
      rest: self.rest.resolve(snap),
    }
  }

  pub(super) fn configs(&self) -> Vec<&ConfigDefinition> {
    [
      self.initial_pwm_length.config_display(),
      self.initial_pwm_power.config_display(),
      self.secondary_pwm_length.config_display(),
      self.secondary_pwm_power.config_display(),
      self.rest.config_display(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}

#[derive(Clone, Debug, Default)]
pub struct PulseModeBuilder {
  mode: PulseMode,
}

impl PulseModeBuilder {
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
    DriverMode::Pulse(self.mode)
  }
}
