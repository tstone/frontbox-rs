use std::time::Duration;

use fast_protocol::{DriverConfig, Power};

use crate::operator_config::{GeneralizedConfigValue, HardwareValue};
use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct FlipperMain3TryMode {
  pub button_switch: &'static str,
  pub invert_button_switch: Option<bool>,
  pub eos_switch: &'static str,
  pub max_on_time: HardwareValue<Duration>,
  pub eos_hold_pwm_power: HardwareValue<Power>,
  pub rest: HardwareValue<Duration>,
}

impl FlipperMain3TryMode {
  fn new(button_switch: &'static str, eos_switch: &'static str) -> Self {
    Self {
      button_switch,
      invert_button_switch: None,
      eos_switch,
      max_on_time: HardwareValue::Fixed(Duration::from_millis(150)),
      eos_hold_pwm_power: HardwareValue::Fixed(Power::QUARTER),
      rest: HardwareValue::Fixed(Duration::from_millis(50))
    }
  }

  pub(super) fn to_config(&self, ctx: &BootSnapshot) -> DriverConfig {
    DriverConfig::FlipperMain3Try {
      button_switch: ctx
        .switches
        .by_name(self.button_switch)
        .map(|sw| sw.id)
        .expect("Flipper main direct mode requires a valid button switch"),
      invert_button_switch: self.invert_button_switch,
      eos_switch: ctx
        .switches
        .by_name(self.eos_switch)
        .map(|sw| sw.id)
        .expect("Flipper main direct mode requires a valid EOS switch"),
      max_on_time: self.max_on_time.resolve(&ctx.operator_config),
      eos_hold_pwm_power: self.eos_hold_pwm_power.resolve(&ctx.operator_config),
      rest: self.rest.resolve(&ctx.operator_config),
    }
  }

  pub(super) fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    [
      self.max_on_time.generalized_config_value(),
      self.eos_hold_pwm_power.generalized_config_value(),
      self.rest.generalized_config_value(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}

/// Switches are required, so this builder is created with them via `DriverMode::flipper_main_direct(...)`
#[derive(Clone, Debug)]
pub struct FlipperMain3TryModeBuilder {
  mode: FlipperMain3TryMode,
}

impl FlipperMain3TryModeBuilder {
  pub(super) fn new(button_switch: &'static str, eos_switch: &'static str) -> Self {
    Self {
      mode: FlipperMain3TryMode::new(button_switch, eos_switch),
    }
  }

  pub fn invert_button_switch(mut self, invert: bool) -> Self {
    self.mode.invert_button_switch = Some(invert);
    self
  }

  pub fn max_on_time(mut self, value: impl Into<HardwareValue<Duration>>) -> Self {
    self.mode.max_on_time = value.into();
    self
  }

  pub fn eos_hold_pwm_power(mut self, value: impl Into<HardwareValue<Power>>) -> Self {
    self.mode.eos_hold_pwm_power = value.into();
    self
  }

  pub fn rest(mut self, value: impl Into<HardwareValue<Duration>>) -> Self {
    self.mode.rest = value.into();
    self
  }

  pub fn build(self) -> DriverMode {
    DriverMode::FlipperMain3Try(self.mode)
  }
}
