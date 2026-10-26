use std::time::Duration;

use fast_protocol::{DriverConfig, Power};

use crate::hardware::io::driver_switches::*;
use crate::operator_config::{ConfigDefinition, HardwareValue};
use crate::prelude::*;

/// Mode 78 - Pulse, then hold for a timed period. Re-triggering the driver while it is holding (e.g. an
/// `ActivationMode::Tap`) resets the hold back to the full `secondary_pwm_length` rather than restarting the pulse.
/// This acts as a watchdog: the driver turns off on its own once re-triggers stop arriving, which is useful for
/// things like a gate that should stay open only while software keeps asking for it.
/// <https://fastpinball.com/fast-serial-protocol/net/driver-mode/78/>
#[derive(Clone, Debug, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PulseHoldExtensionMode {
  /// What causes the driver to fire (be triggered)
  pub trigger_mode: DriverTriggerMode,
  pub initial_pwm_length: HardwareValue<Duration>,
  pub initial_pwm_power: HardwareValue<Power>,
  /// How long the driver holds after the initial pulse, and what each re-trigger resets it to. 10ms resolution, up
  /// to 2.55s.
  pub secondary_pwm_length: HardwareValue<Duration>,
  pub secondary_pwm_power: HardwareValue<Power>,
  /// Time after the driver goes off before it can be triggered again
  pub rest: HardwareValue<Duration>,
}

impl Default for PulseHoldExtensionMode {
  fn default() -> Self {
    Self {
      trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
      initial_pwm_length: HardwareValue::Fixed(Duration::from_millis(30)),
      initial_pwm_power: HardwareValue::Fixed(Power::FULL),
      secondary_pwm_length: HardwareValue::Fixed(Duration::from_millis(1000)),
      secondary_pwm_power: HardwareValue::Fixed(Power::QUARTER),
      rest: HardwareValue::Fixed(Duration::ZERO),
    }
  }
}

impl PulseHoldExtensionMode {
  pub(super) fn to_config(&self, snap: &BootSnapshot) -> DriverConfig {
    let (switch, invert_switch) = get_switch_id_and_invert(&self.trigger_mode, snap);

    DriverConfig::PulseHoldExtension {
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
pub struct PulseHoldExtensionModeBuilder {
  mode: PulseHoldExtensionMode,
}

impl PulseHoldExtensionModeBuilder {
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

  /// How long the driver holds after the initial pulse, and what each re-trigger resets it to. 10ms resolution, up
  /// to 2.55s.
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
    DriverMode::PulseHoldExtension(self.mode)
  }
}
