use std::fmt::Debug;
use std::time::Duration;

use dyn_clone::DynClone;
use fast_protocol::{DriverConfig, Power};

use crate::hardware::io::driver_switches::*;
use crate::operator_config::{GeneralizedConfigValue, HardwareValue};
use crate::prelude::ContextBase;
use crate::{DriverTriggerDualMode, DriverTriggerMode};

/// DriverMode is a wrapper around DriverConfig that allows these features:
/// 1. Referencing switches by name instead of index, which avoids having to calculate ID offsets
/// 2. Allows use of ..Default::default() since DriverConfig is an enum
pub trait DriverMode: DynClone + Debug + Send + Sync {
  fn to_config(&self, ctx: &ContextBase) -> DriverConfig;
  fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue>;
}

dyn_clone::clone_trait_object!(DriverMode);

/// Mode 10 - Pulse the driver, up to 255ms, when triggered.
/// <https://fastpinball.com/fast-serial-protocol/net/driver-mode/10/>
#[derive(Debug, Clone)]
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

impl DriverMode for PulseMode {
  fn to_config(&self, ctx: &ContextBase) -> DriverConfig {
    let (switch, invert_switch) = get_switch_id_and_invert(&self.trigger_mode, ctx);

    DriverConfig::Pulse {
      switch,
      invert_switch,
      initial_pwm_length: self.initial_pwm_length.resolve(&ctx.operator_config),
      initial_pwm_power: self.initial_pwm_power.resolve(&ctx.operator_config),
      secondary_pwm_length: self.secondary_pwm_length.resolve(&ctx.operator_config),
      secondary_pwm_power: self.secondary_pwm_power.resolve(&ctx.operator_config),
      rest: self.rest.resolve(&ctx.operator_config),
    }
  }

  fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    vec![
      self.initial_pwm_length.config_value(),
      self.initial_pwm_power.config_value(),
      self.secondary_pwm_length.config_value(),
      self.secondary_pwm_power.config_value(),
      self.rest.config_value(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}

/// Mode 12 - Sends up to 2 variable PWM times, then kicks (full power) at the end of the cycle. Useful for gently
/// moving a coil and then kicking it the rest of the way, e.g. VUK or trough eject. Reduces force applied
/// to ball by ensuring a plunger has full contact with the ball before a full kick occurs.
/// <https://fastpinball.com/fast-serial-protocol/net/driver-mode/12/>
#[derive(Debug, Clone)]
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

impl DriverMode for PulseKickMode {
  fn to_config(&self, ctx: &ContextBase) -> DriverConfig {
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

  fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    vec![
      self.initial_pwm_length.config_value(),
      self.initial_pwm_power.config_value(),
      self.secondary_pwm_length.config_value(),
      self.secondary_pwm_power.config_value(),
      self.kick_length.config_value(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}

/// Mode 18 - Holds a driver in the on state as long as the trigger is active. An initial PWM can be configured
/// before the long hold.
/// <https://fastpinball.com/fast-serial-protocol/net/driver-mode/18/>
#[derive(Debug, Clone)]
pub struct PulseHoldMode {
  /// What causes the driver to fire (be triggered)
  pub trigger_mode: DriverTriggerMode,
  pub initial_pwm_length: HardwareValue<Duration>,
  pub initial_pwm_power: HardwareValue<Power>,
  pub secondary_pwm_power: HardwareValue<Power>,
  /// Time after the driver goes off before it can be triggered again
  pub rest: HardwareValue<Duration>,
}

impl Default for PulseHoldMode {
  fn default() -> Self {
    Self {
      trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
      initial_pwm_length: HardwareValue::Fixed(Duration::from_millis(30)),
      initial_pwm_power: HardwareValue::Fixed(Power::FULL),
      secondary_pwm_power: HardwareValue::Fixed(Power::ZERO),
      rest: HardwareValue::Fixed(Duration::ZERO),
    }
  }
}

impl DriverMode for PulseHoldMode {
  fn to_config(&self, ctx: &ContextBase) -> DriverConfig {
    let (switch, invert_switch) = get_switch_id_and_invert(&self.trigger_mode, ctx);

    DriverConfig::PulseHold {
      switch,
      invert_switch,
      initial_pwm_length: self.initial_pwm_length.resolve(&ctx.operator_config),
      initial_pwm_power: self.initial_pwm_power.resolve(&ctx.operator_config),
      secondary_pwm_power: self.secondary_pwm_power.resolve(&ctx.operator_config),
      rest: self.rest.resolve(&ctx.operator_config),
    }
  }

  fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    vec![
      self.initial_pwm_length.config_value(),
      self.initial_pwm_power.config_value(),
      self.secondary_pwm_power.config_value(),
      self.rest.config_value(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}

/// Mode 20 - Pulse then indefinitely hold the driver on until the trigger (flip) is deactivated -OR- the cancel
/// switch (flop) is activated.
/// <https://fastpinball.com/fast-serial-protocol/net/driver-mode/20/>
#[derive(Debug, Clone)]
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

impl DriverMode for PulseHoldCancelMode {
  fn to_config(&self, ctx: &ContextBase) -> DriverConfig {
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

  fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    vec![
      self.initial_pwm_length.config_value(),
      self.initial_pwm_power.config_value(),
      self.secondary_pwm_power.config_value(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}

/// Mode 30 - Insert a delay between when the switch is triggered and the driver fires.
/// Useful for things kickbacks where a bit of delay needs to be added into the automatic flow.
/// <https://fastpinball.com/fast-serial-protocol/net/driver-mode/30/>
#[derive(Debug, Clone)]
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

impl DriverMode for DelayedPulseMode {
  fn to_config(&self, ctx: &ContextBase) -> DriverConfig {
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

  fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    vec![
      self.delay_length.config_value(),
      self.initial_full_power_length.config_value(),
      self.secondary_pwm_length.config_value(),
      self.secondary_pwm_power.config_value(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}

/// Mode 20 - Pulse then indefinitely hold the driver on until the trigger (flip) is deactivated -OR- the cancel
/// switch (flop) is activated.
/// <https://fastpinball.com/fast-serial-protocol/net/driver-mode/20/>
#[derive(Debug, Clone)]
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

impl DriverMode for PulseCancelMode {
  fn to_config(&self, ctx: &ContextBase) -> DriverConfig {
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

  fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    vec![
      self.initial_full_power_length.config_value(),
      self.secondary_power_length.config_value(),
      self.secondary_pwm_power.config_value(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}

/// Mode 70 - Pulse the driver for an initial time (up to 255ms), then hold it for a secondary time (up to 25s).
/// <https://fastpinball.com/fast-serial-protocol/net/driver-mode/70/>
#[derive(Debug, Clone)]
pub struct LongPulseMode {
  /// What causes the driver to fire (be triggered)
  pub trigger_mode: DriverTriggerMode,
  pub initial_pwm_length: HardwareValue<Duration>,
  pub initial_pwm_power: HardwareValue<Power>,
  pub secondary_pwm_length: HardwareValue<Duration>,
  pub secondary_pwm_power: HardwareValue<Power>,
  /// Time after the driver goes off before it can be triggered again
  pub rest: HardwareValue<Duration>,
}

impl Default for LongPulseMode {
  fn default() -> Self {
    Self {
      trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
      initial_pwm_length: HardwareValue::Fixed(Duration::from_millis(200)),
      initial_pwm_power: HardwareValue::Fixed(Power::FULL),
      secondary_pwm_length: HardwareValue::Fixed(Duration::from_millis(1000)),
      secondary_pwm_power: HardwareValue::Fixed(Power::QUARTER),
      rest: HardwareValue::Fixed(Duration::from_millis(255)),
    }
  }
}

impl DriverMode for LongPulseMode {
  fn to_config(&self, ctx: &ContextBase) -> DriverConfig {
    let (switch, invert_switch) = get_switch_id_and_invert(&self.trigger_mode, ctx);

    DriverConfig::LongPulse {
      switch,
      invert_switch,
      initial_pwm_length: self.initial_pwm_length.resolve(&ctx.operator_config),
      initial_pwm_power: self.initial_pwm_power.resolve(&ctx.operator_config),
      secondary_pwm_length: self.secondary_pwm_length.resolve(&ctx.operator_config),
      secondary_pwm_power: self.secondary_pwm_power.resolve(&ctx.operator_config),
      rest: self.rest.resolve(&ctx.operator_config),
    }
  }

  fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    vec![
      self.initial_pwm_length.config_value(),
      self.initial_pwm_power.config_value(),
      self.secondary_pwm_length.config_value(),
      self.secondary_pwm_power.config_value(),
      self.rest.config_value(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}

/// Mode 80 - Premium flipper driver for main coil. Driver is active when button switch is closed.
#[derive(Debug, Clone)]
pub struct FlipperMainDirectMode {
  pub button_switch: &'static str,
  pub invert_button_switch: Option<bool>,
  pub eos_switch: &'static str,
  pub initial_pwm_power: HardwareValue<Power>,
  pub secondary_pwm_power: HardwareValue<Power>,
  pub max_eos_time: HardwareValue<Duration>,
  pub next_flip_refresh: HardwareValue<Duration>,
}

impl Default for FlipperMainDirectMode {
  fn default() -> Self {
    Self {
      button_switch: "",
      invert_button_switch: None,
      eos_switch: "",
      initial_pwm_power: HardwareValue::Fixed(Power::percent(45)),
      secondary_pwm_power: HardwareValue::Fixed(Power::FULL),
      max_eos_time: HardwareValue::Fixed(Duration::from_millis(60)),
      next_flip_refresh: HardwareValue::Fixed(Duration::from_millis(8)),
    }
  }
}

impl DriverMode for FlipperMainDirectMode {
  fn to_config(&self, ctx: &ContextBase) -> DriverConfig {
    DriverConfig::FlipperMainDirect {
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
      initial_pwm_power: self.initial_pwm_power.resolve(&ctx.operator_config),
      secondary_pwm_power: self.secondary_pwm_power.resolve(&ctx.operator_config),
      max_eos_time: self.max_eos_time.resolve(&ctx.operator_config),
      next_flip_refresh: self.next_flip_refresh.resolve(&ctx.operator_config),
    }
  }

  fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    vec![
      self.initial_pwm_power.config_value(),
      self.secondary_pwm_power.config_value(),
      self.max_eos_time.config_value(),
      self.next_flip_refresh.config_value(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}

/// Mode 81 - Premium flipper driver for hold coil
#[derive(Debug, Clone)]
pub struct FlipperHoldDirectMode {
  pub button_switch: &'static str,
  pub invert_button_switch: Option<bool>,
  pub driver_on_time: HardwareValue<Duration>,
  pub initial_pwm_power: HardwareValue<Power>,
  pub secondary_pwm_power: HardwareValue<Power>,
}

impl Default for FlipperHoldDirectMode {
  fn default() -> Self {
    Self {
      button_switch: "",
      invert_button_switch: None,
      driver_on_time: HardwareValue::Fixed(Duration::from_millis(48)),
      initial_pwm_power: HardwareValue::Fixed(Power::FULL),
      secondary_pwm_power: HardwareValue::Fixed(Power::FULL),
    }
  }
}

impl DriverMode for FlipperHoldDirectMode {
  fn to_config(&self, ctx: &ContextBase) -> DriverConfig {
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

  fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    vec![
      self.driver_on_time.config_value(),
      self.initial_pwm_power.config_value(),
      self.secondary_pwm_power.config_value(),
    ]
    .into_iter()
    .flatten()
    .collect()
  }
}
