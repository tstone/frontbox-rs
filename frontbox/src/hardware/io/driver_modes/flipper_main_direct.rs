use std::time::Duration;

use fast_protocol::{DriverConfig, Power};

use crate::operator_config::{ConfigDefinition, HardwareValue};
use crate::prelude::*;

/// Mode 80 - Premium flipper driver for main coil. Driver is active when button switch is closed.
#[derive(Clone, Debug, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FlipperMainDirectMode {
  pub button_switch: &'static str,
  pub invert_button_switch: Option<bool>,
  pub eos_switch: &'static str,
  pub initial_pwm_power: HardwareValue<Power>,
  pub secondary_pwm_power: HardwareValue<Power>,
  pub max_eos_time: HardwareValue<Duration>,
  pub next_flip_refresh: HardwareValue<Duration>,
}

impl FlipperMainDirectMode {
  fn new(button_switch: &'static str, eos_switch: &'static str) -> Self {
    Self {
      button_switch,
      invert_button_switch: None,
      eos_switch,
      initial_pwm_power: HardwareValue::Fixed(Power::FULL),
      secondary_pwm_power: HardwareValue::Fixed(Power::HALF),
      max_eos_time: HardwareValue::Fixed(Duration::from_millis(60)),
      next_flip_refresh: HardwareValue::Fixed(Duration::from_millis(8)),
    }
  }

  pub(super) fn to_config(&self, snap: &BootSnapshot) -> DriverConfig {
    DriverConfig::FlipperMainDirect {
      button_switch: snap
        .switches
        .by_name(self.button_switch)
        .map(|sw| sw.id)
        .expect("Flipper main direct mode requires a valid button switch"),
      invert_button_switch: self.invert_button_switch,
      eos_switch: snap
        .switches
        .by_name(self.eos_switch)
        .map(|sw| sw.id)
        .expect("Flipper main direct mode requires a valid EOS switch"),
      initial_pwm_power: self.initial_pwm_power.resolve(snap),
      secondary_pwm_power: self.secondary_pwm_power.resolve(snap),
      max_eos_time: self.max_eos_time.resolve(snap),
      next_flip_refresh: self.next_flip_refresh.resolve(snap),
    }
  }

  pub(super) fn configs(&self) -> Vec<&ConfigDefinition> {
    [
      self.initial_pwm_power.config_display(),
      self.secondary_pwm_power.config_display(),
      self.max_eos_time.config_display(),
      self.next_flip_refresh.config_display(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}

/// Switches are required, so this builder is created with them via `DriverMode::flipper_main_direct(...)`
#[derive(Clone, Debug)]
pub struct FlipperMainDirectModeBuilder {
  mode: FlipperMainDirectMode,
}

impl FlipperMainDirectModeBuilder {
  pub(super) fn new(button_switch: &'static str, eos_switch: &'static str) -> Self {
    Self {
      mode: FlipperMainDirectMode::new(button_switch, eos_switch),
    }
  }

  pub fn invert_button_switch(mut self, invert: bool) -> Self {
    self.mode.invert_button_switch = Some(invert);
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

  pub fn max_eos_time(mut self, value: impl Into<HardwareValue<Duration>>) -> Self {
    self.mode.max_eos_time = value.into();
    self
  }

  pub fn next_flip_refresh(mut self, value: impl Into<HardwareValue<Duration>>) -> Self {
    self.mode.next_flip_refresh = value.into();
    self
  }

  pub fn build(self) -> DriverMode {
    DriverMode::FlipperMainDirect(self.mode)
  }
}
