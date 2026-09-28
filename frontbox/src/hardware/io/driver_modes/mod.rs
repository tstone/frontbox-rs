mod delayed_pulse;
mod flipper_hold_direct;
mod flipper_main_direct;
mod long_pulse;
mod pulse;
mod pulse_cancel;
mod pulse_hold;
mod pulse_hold_cancel;
mod pulse_kick;

pub use delayed_pulse::*;
pub use flipper_hold_direct::*;
pub use flipper_main_direct::*;
pub use long_pulse::*;
pub use pulse::*;
pub use pulse_cancel::*;
pub use pulse_hold::*;
pub use pulse_hold_cancel::*;
pub use pulse_kick::*;

use fast_protocol::DriverConfig;

use crate::operator_config::GeneralizedConfigValue;
use crate::prelude::*;

/// DriverMode is a wrapper around DriverConfig that allows these features:
/// 1. Referencing switches by name instead of index, which avoids having to calculate ID offsets
/// 2. Sensible defaults for every mode, overridable via builders (e.g. `DriverMode::pulse().rest(...).build()`)
/// 3. Hardware values that can be exposed through operator config
#[derive(Clone, Debug)]
pub enum DriverMode {
  Pulse(PulseMode),
  PulseKick(PulseKickMode),
  PulseHold(PulseHoldMode),
  PulseHoldCancel(PulseHoldCancelMode),
  DelayedPulse(DelayedPulseMode),
  PulseCancel(PulseCancelMode),
  LongPulse(LongPulseMode),
  FlipperMainDirect(FlipperMainDirectMode),
  FlipperHoldDirect(FlipperHoldDirectMode),
}

impl DriverMode {
  /// Mode 10 - Pulse the driver, up to 255ms, when triggered.
  pub fn pulse() -> PulseModeBuilder {
    PulseModeBuilder::default()
  }

  /// Mode 12 - Up to 2 variable PWM times, then a full power kick at the end of the cycle.
  pub fn pulse_kick() -> PulseKickModeBuilder {
    PulseKickModeBuilder::default()
  }

  /// Mode 18 - Hold the driver on as long as the trigger is active, with an optional initial PWM.
  pub fn pulse_hold() -> PulseHoldModeBuilder {
    PulseHoldModeBuilder::default()
  }

  /// Mode 20 - Pulse then hold until the trigger (flip) is deactivated or the cancel switch (flop) is activated.
  pub fn pulse_hold_cancel() -> PulseHoldCancelModeBuilder {
    PulseHoldCancelModeBuilder::default()
  }

  /// Mode 30 - Insert a delay between when the switch is triggered and the driver fires.
  pub fn delayed_pulse() -> DelayedPulseModeBuilder {
    DelayedPulseModeBuilder::default()
  }

  /// Mode 75 - Pulse until the trigger (flip) is deactivated or the cancel switch (flop) is activated.
  pub fn pulse_cancel() -> PulseCancelModeBuilder {
    PulseCancelModeBuilder::default()
  }

  /// Mode 70 - Pulse for an initial time (up to 255ms), then hold for a secondary time (up to 25s).
  pub fn long_pulse() -> LongPulseModeBuilder {
    LongPulseModeBuilder::default()
  }

  /// Mode 80 - Premium flipper driver for main coil.
  pub fn flipper_main_direct(
    button_switch: &'static str,
    eos_switch: &'static str,
  ) -> FlipperMainDirectModeBuilder {
    FlipperMainDirectModeBuilder::new(button_switch, eos_switch)
  }

  /// Mode 81 - Premium flipper driver for hold coil.
  pub fn flipper_hold_direct(button_switch: &'static str) -> FlipperHoldDirectModeBuilder {
    FlipperHoldDirectModeBuilder::new(button_switch)
  }

  pub fn to_config(&self, ctx: &BootSnapshot) -> DriverConfig {
    match self {
      Self::Pulse(m) => m.to_config(ctx),
      Self::PulseKick(m) => m.to_config(ctx),
      Self::PulseHold(m) => m.to_config(ctx),
      Self::PulseHoldCancel(m) => m.to_config(ctx),
      Self::DelayedPulse(m) => m.to_config(ctx),
      Self::PulseCancel(m) => m.to_config(ctx),
      Self::LongPulse(m) => m.to_config(ctx),
      Self::FlipperMainDirect(m) => m.to_config(ctx),
      Self::FlipperHoldDirect(m) => m.to_config(ctx),
    }
  }

  pub fn generalized_config_values(&self) -> Vec<&dyn GeneralizedConfigValue> {
    match self {
      Self::Pulse(m) => m.generalized_config_values(),
      Self::PulseKick(m) => m.generalized_config_values(),
      Self::PulseHold(m) => m.generalized_config_values(),
      Self::PulseHoldCancel(m) => m.generalized_config_values(),
      Self::DelayedPulse(m) => m.generalized_config_values(),
      Self::PulseCancel(m) => m.generalized_config_values(),
      Self::LongPulse(m) => m.generalized_config_values(),
      Self::FlipperMainDirect(m) => m.generalized_config_values(),
      Self::FlipperHoldDirect(m) => m.generalized_config_values(),
    }
  }
}
