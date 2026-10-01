use std::time::Duration;

use fast_protocol::{DriverConfig, Power};

use crate::operator_config::{GeneralizedConfigValue, HardwareValue};
use crate::prelude::*;

/// Mode 81 - Premium flipper driver for hold coil
#[derive(Clone, Debug, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FlipperHoldDirectMode {
  pub button_switch: &'static str,
  pub invert_button_switch: Option<bool>,
  pub driver_on_time: HardwareValue<Duration>,
  pub initial_pwm_power: HardwareValue<Power>,
  pub secondary_pwm_power: HardwareValue<Power>,
}

impl FlipperHoldDirectMode {
  fn new(button_switch: &'static str) -> Self {
    Self {
      button_switch,
      invert_button_switch: None,
      driver_on_time: HardwareValue::Fixed(Duration::from_millis(48)),
      initial_pwm_power: HardwareValue::Fixed(Power::FULL),
      secondary_pwm_power: HardwareValue::Fixed(Power::FULL),
    }
  }

  pub(super) fn to_config(&self, ctx: &BootSnapshot) -> DriverConfig {
    DriverConfig::FlipperHoldDirect {
      button_switch: ctx
        .switches
        .by_name(self.button_switch)
        .map(|sw| sw.id)
        .expect("Flipper hold direct mode requires a valid button switch"),
      invert_button_switch: self.invert_button_switch,
      driver_on_time: self.driver_on_time.resolve(&ctx.operator_config),
      initial_pwm_power: self.initial_pwm_power.resolve(&ctx.operator_config),
      secondary_pwm_power: self.secondary_pwm_power.resolve(&ctx.operator_config),
    }
  }

  pub(super) fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    [
      self.driver_on_time.generalized_config_value(),
      self.initial_pwm_power.generalized_config_value(),
      self.secondary_pwm_power.generalized_config_value(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}

/// The button switch is required, so this builder is created with it via `DriverMode::flipper_hold_direct(...)`
#[derive(Clone, Debug)]
pub struct FlipperHoldDirectModeBuilder {
  mode: FlipperHoldDirectMode,
}

impl FlipperHoldDirectModeBuilder {
  pub(super) fn new(button_switch: &'static str) -> Self {
    Self {
      mode: FlipperHoldDirectMode::new(button_switch),
    }
  }

  pub fn invert_button_switch(mut self, invert: bool) -> Self {
    self.mode.invert_button_switch = Some(invert);
    self
  }

  pub fn driver_on_time(mut self, value: impl Into<HardwareValue<Duration>>) -> Self {
    self.mode.driver_on_time = value.into();
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

  pub fn build(self) -> DriverMode {
    DriverMode::FlipperHoldDirect(self.mode)
  }
}
