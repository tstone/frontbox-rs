use std::collections::HashMap;
use std::time::Duration;

use fast_protocol::{DriverConfig, Power};

use crate::{DriverTriggerDualMode, DriverTriggerMode};

/// DriverMode is a wrapper around DriverConfig that allows these features:
/// 1. Referencing switches by name instead of index, which avoids having to calculate ID offsets
/// 2. Allows use of ..Default::default() since DriverConfig is an enum
pub trait DriverMode {
  fn to_config(&self, switch_lookup: &dyn SwitchLookup) -> DriverConfig;
}

/// Mode 10 - Pulse the driver, up to 255ms, when triggered.
/// https://fastpinball.com/fast-serial-protocol/net/driver-mode/10/
#[derive(Debug, Clone)]
pub struct PulseMode {
  /// What causes the driver to fire (be triggered)
  pub trigger_mode: DriverTriggerMode,
  pub initial_pwm_length: Duration,
  pub initial_pwm_power: Power,
  pub secondary_pwm_length: Duration,
  pub secondary_pwm_power: Power,
  /// Time after the driver goes off before it can be triggered again
  pub rest: Duration,
}

impl Default for PulseMode {
  fn default() -> Self {
    Self {
      trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
      initial_pwm_length: Duration::from_millis(20),
      initial_pwm_power: Power::FULL,
      secondary_pwm_length: Duration::ZERO,
      secondary_pwm_power: Power::ZERO,
      rest: Duration::from_millis(80),
    }
  }
}

impl DriverMode for PulseMode {
  fn to_config(&self, switch_lookup: &dyn SwitchLookup) -> DriverConfig {
    let (switch, invert_switch) = get_switch_invert(&self.trigger_mode, switch_lookup);

    DriverConfig::Pulse {
      switch,
      invert_switch,
      initial_pwm_length: self.initial_pwm_length,
      initial_pwm_power: self.initial_pwm_power,
      secondary_pwm_length: self.secondary_pwm_length,
      secondary_pwm_power: self.secondary_pwm_power,
      rest: self.rest,
    }
  }
}

pub trait SwitchLookup {
  fn get_switch_id(&self, name: &str) -> Option<usize>;
}

impl SwitchLookup for HashMap<&'static str, usize> {
  fn get_switch_id(&self, name: &str) -> Option<usize> {
    self.get(name).copied()
  }
}

/// Mode 12 - Sends up to 2 variable PWM times, then kicks (full power) at the end of the cycle. Useful for gently
/// moving a coil and then kicking it the rest of the way, e.g. VUK or trough eject. Reduces force applied
/// to ball by ensuring a plunger has full contact with the ball before a full kick occurs.
/// https://fastpinball.com/fast-serial-protocol/net/driver-mode/12/
#[derive(Debug, Clone)]
pub struct PulseKickMode {
  /// What causes the driver to fire (be triggered)
  pub trigger_mode: DriverTriggerMode,
  pub initial_pwm_length: Duration,
  pub initial_pwm_power: Power,
  pub secondary_pwm_length: Duration,
  pub secondary_pwm_power: Power,
  /// Time that the driver is held at full power after the initial and secondary PWM times
  pub kick_length: Duration,
}

impl Default for PulseKickMode {
  fn default() -> Self {
    Self {
      trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
      initial_pwm_length: Duration::from_millis(30),
      initial_pwm_power: Power::FULL,
      secondary_pwm_length: Duration::ZERO,
      secondary_pwm_power: Power::ZERO,
      kick_length: Duration::from_millis(500),
    }
  }
}

impl DriverMode for PulseKickMode {
  fn to_config(&self, switch_lookup: &dyn SwitchLookup) -> DriverConfig {
    let (switch, invert_switch) = get_switch_invert(&self.trigger_mode, switch_lookup);

    DriverConfig::PulseKick {
      switch,
      invert_switch,
      initial_pwm_length: self.initial_pwm_length,
      initial_pwm_power: self.initial_pwm_power,
      secondary_pwm_length: self.secondary_pwm_length,
      secondary_pwm_power: self.secondary_pwm_power,
      kick_length: self.kick_length,
    }
  }
}

/// Mode 18 - Holds a driver in the on state as long as the trigger is active. An initial PWM can be configured
/// before the long hold.
/// https://fastpinball.com/fast-serial-protocol/net/driver-mode/18/
#[derive(Debug, Clone)]
pub struct PulseHoldMode {
  /// What causes the driver to fire (be triggered)
  pub trigger_mode: DriverTriggerMode,
  pub initial_pwm_length: Duration,
  pub initial_pwm_power: Power,
  pub secondary_pwm_power: Power,
  /// Time after the driver goes off before it can be triggered again
  pub rest: Duration,
}

impl Default for PulseHoldMode {
  fn default() -> Self {
    Self {
      trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
      initial_pwm_length: Duration::from_millis(30),
      initial_pwm_power: Power::FULL,
      secondary_pwm_power: Power::ZERO,
      rest: Duration::ZERO,
    }
  }
}

impl DriverMode for PulseHoldMode {
  fn to_config(&self, switch_lookup: &dyn SwitchLookup) -> DriverConfig {
    let (switch, invert_switch) = get_switch_invert(&self.trigger_mode, switch_lookup);

    DriverConfig::PulseHold {
      switch,
      invert_switch,
      initial_pwm_length: self.initial_pwm_length,
      initial_pwm_power: self.initial_pwm_power,
      secondary_pwm_power: self.secondary_pwm_power,
      rest: self.rest,
    }
  }
}

/// Mode 20 - Pulse then indefinitely hold the driver on until the trigger (flip) is deactivated -OR- the cancel
/// switch (flop) is activated.
/// https://fastpinball.com/fast-serial-protocol/net/driver-mode/20/
#[derive(Debug, Clone)]
pub struct PulseHoldCancelMode {
  /// What causes the driver to fire (be triggered)
  pub trigger_mode: DriverTriggerDualMode,
  pub initial_pwm_length: Duration,
  pub secondary_pwm_power: Power,
  pub secondary_pwm_length: Duration,
  /// Time after the driver goes off before it can be triggered again
  pub rest: Duration,
}

impl Default for PulseHoldCancelMode {
  fn default() -> Self {
    Self {
      trigger_mode: DriverTriggerDualMode::Disabled,
      initial_pwm_length: Duration::from_millis(30),
      secondary_pwm_power: Power::percent(10),
      secondary_pwm_length: Duration::from_millis(500),
      rest: Duration::from_millis(500),
    }
  }
}

impl DriverMode for PulseHoldCancelMode {
  fn to_config(&self, switch_lookup: &dyn SwitchLookup) -> DriverConfig {
    let (flip_switch, invert_flip_switch, flop_switch, invert_flop_switch) =
      get_switches_inverts(&self.trigger_mode, switch_lookup);

    DriverConfig::PulseHoldCancel {
      switch: flip_switch,
      invert_switch: invert_flip_switch,
      off_switch: flop_switch,
      invert_off_switch: invert_flop_switch,
      initial_pwm_length: self.initial_pwm_length,
      secondary_pwm_power: self.secondary_pwm_power,
      secondary_pwm_length: self.secondary_pwm_length,
      rest: self.rest,
    }
  }
}

/// Mode 70 - Pulse the driver for an initial time (up to 255ms), then hold it for a secondary time (up to 25s).
/// https://fastpinball.com/fast-serial-protocol/net/driver-mode/70/
#[derive(Debug, Clone)]
pub struct LongPulseMode {
  /// What causes the driver to fire (be triggered)
  pub trigger_mode: DriverTriggerMode,
  pub initial_pwm_length: Duration,
  pub initial_pwm_power: Power,
  pub secondary_pwm_length: Duration,
  pub secondary_pwm_power: Power,
  /// Time after the driver goes off before it can be triggered again
  pub rest: Duration,
}

impl Default for LongPulseMode {
  fn default() -> Self {
    Self {
      trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
      initial_pwm_length: Duration::from_millis(200),
      initial_pwm_power: Power::FULL,
      secondary_pwm_length: Duration::from_millis(1000),
      secondary_pwm_power: Power::percent(25),
      rest: Duration::from_millis(1000),
    }
  }
}

impl DriverMode for LongPulseMode {
  fn to_config(&self, switch_lookup: &dyn SwitchLookup) -> DriverConfig {
    let (switch, invert_switch) = get_switch_invert(&self.trigger_mode, switch_lookup);

    DriverConfig::LongPulse {
      switch,
      invert_switch,
      initial_pwm_length: self.initial_pwm_length,
      initial_pwm_power: self.initial_pwm_power,
      secondary_pwm_length: self.secondary_pwm_length,
      secondary_pwm_power: self.secondary_pwm_power,
      rest: self.rest,
    }
  }
}

/// Mode 80 - Premium flipper driver for main coil. Driver is active when button switch is closed.
#[derive(Debug, Clone)]
pub struct FlipperMainDirectMode {
  pub button_switch: &'static str,
  pub invert_button_switch: Option<bool>,
  pub eos_switch: &'static str,
  pub initial_pwm_power: Power,
  pub secondary_pwm_power: Power,
  pub max_eos_time: Duration,
  pub next_flip_refresh: Duration,
}

impl Default for FlipperMainDirectMode {
  fn default() -> Self {
    Self {
      button_switch: "",
      invert_button_switch: None,
      eos_switch: "",
      initial_pwm_power: Power::percent(45),
      secondary_pwm_power: Power::FULL,
      max_eos_time: Duration::from_millis(60),
      next_flip_refresh: Duration::from_millis(8),
    }
  }
}

impl DriverMode for FlipperMainDirectMode {
  fn to_config(&self, switch_lookup: &dyn SwitchLookup) -> DriverConfig {
    DriverConfig::FlipperMainDirect {
      button_switch: switch_lookup
        .get_switch_id(self.button_switch)
        .expect("Flipper main direct mode requires a valid button switch"),
      invert_button_switch: self.invert_button_switch,
      eos_switch: switch_lookup
        .get_switch_id(self.eos_switch)
        .expect("Flipper main direct mode requires a valid EOS switch"),
      initial_pwm_power: self.initial_pwm_power,
      secondary_pwm_power: self.secondary_pwm_power,
      max_eos_time: self.max_eos_time,
      next_flip_refresh: self.next_flip_refresh,
    }
  }
}

/// Mode 81 - Premium flipper driver for hold coil
#[derive(Debug, Clone)]
pub struct FlipperHoldDirectMode {
  pub button_switch: &'static str,
  pub invert_button_switch: Option<bool>,
  pub driver_on_time: Duration,
  pub initial_pwm_power: Power,
  pub secondary_pwm_power: Power,
}

impl Default for FlipperHoldDirectMode {
  fn default() -> Self {
    Self {
      button_switch: "",
      invert_button_switch: None,
      driver_on_time: Duration::from_millis(48),
      initial_pwm_power: Power::FULL,
      secondary_pwm_power: Power::FULL,
    }
  }
}

impl DriverMode for FlipperHoldDirectMode {
  fn to_config(&self, switch_lookup: &dyn SwitchLookup) -> DriverConfig {
    DriverConfig::FlipperHoldDirect {
      button_switch: switch_lookup
        .get_switch_id(self.button_switch)
        .expect("Flipper hold direct mode requires a valid button switch"),
      invert_button_switch: self.invert_button_switch,
      driver_on_time: self.driver_on_time,
      initial_pwm_power: self.initial_pwm_power,
      secondary_pwm_power: self.secondary_pwm_power,
    }
  }
}

fn get_switch_invert(
  trigger_mode: &DriverTriggerMode,
  switch_lookup: &dyn SwitchLookup,
) -> (Option<usize>, Option<bool>) {
  match trigger_mode {
    DriverTriggerMode::Disabled => (None, None),
    DriverTriggerMode::Switch(s) => (switch_lookup.get_switch_id(s), Some(false)),
    DriverTriggerMode::InvertedSwitch(s) => (switch_lookup.get_switch_id(s), Some(true)),
    DriverTriggerMode::VirtualSwitchTrue => (None, Some(false)),
    DriverTriggerMode::VirtualSwitchFalse => (None, Some(true)),
  }
}

fn get_switches_inverts(
  trigger_mode: &DriverTriggerDualMode,
  switch_lookup: &dyn SwitchLookup,
) -> (Option<usize>, Option<bool>, Option<usize>, Option<bool>) {
  match trigger_mode {
    DriverTriggerDualMode::Disabled => (None, None, None, None),
    DriverTriggerDualMode::FlipSwitchTrue_FlopSwitchTrue {
      flip_switch,
      flop_switch,
    } => (
      switch_lookup.get_switch_id(flip_switch),
      Some(false),
      switch_lookup.get_switch_id(flop_switch),
      Some(false),
    ),
    DriverTriggerDualMode::FlipSwitchFalse_FlopSwitchTrue {
      flip_switch,
      flop_switch,
    } => (
      switch_lookup.get_switch_id(flip_switch),
      Some(true),
      switch_lookup.get_switch_id(flop_switch),
      Some(false),
    ),
    DriverTriggerDualMode::FlipSwitchTrue_FlopSwitchFalse {
      flip_switch,
      flop_switch,
    } => (
      switch_lookup.get_switch_id(flip_switch),
      Some(false),
      switch_lookup.get_switch_id(flop_switch),
      Some(true),
    ),
    DriverTriggerDualMode::FlipSwitchFalse_FlopSwitchFalse {
      flip_switch,
      flop_switch,
    } => (
      switch_lookup.get_switch_id(flip_switch),
      Some(true),
      switch_lookup.get_switch_id(flop_switch),
      Some(true),
    ),
    DriverTriggerDualMode::VirtualFlip_FlopSwitchTrue(virtual_flip) => (
      None,
      Some(false),
      switch_lookup.get_switch_id(virtual_flip),
      Some(false),
    ),
    DriverTriggerDualMode::VirtualFlip_FlopSwitchFalse(virtual_flip) => (
      None,
      Some(false),
      switch_lookup.get_switch_id(virtual_flip),
      Some(true),
    ),
    DriverTriggerDualMode::FlipSwitchTrue_VirtualFlop(virtual_flop) => (
      switch_lookup.get_switch_id(virtual_flop),
      Some(false),
      None,
      Some(false),
    ),
    DriverTriggerDualMode::FlipSwitchFalse_VirtualFlop(virtual_flop) => (
      switch_lookup.get_switch_id(virtual_flop),
      Some(true),
      None,
      Some(false),
    ),
  }
}
