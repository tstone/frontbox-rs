mod delayed_pulse;
mod flipper_hold_direct;
mod flipper_main_3try;
mod flipper_main_direct;
mod long_pulse;
mod pulse;
mod pulse_cancel;
mod pulse_hold;
mod pulse_hold_cancel;
mod pulse_hold_extension;
mod pulse_kick;

pub use delayed_pulse::*;
pub use flipper_hold_direct::*;
pub use flipper_main_3try::*;
pub use flipper_main_direct::*;
pub use long_pulse::*;
pub use pulse::*;
pub use pulse_cancel::*;
pub use pulse_hold::*;
pub use pulse_hold_cancel::*;
pub use pulse_hold_extension::*;
pub use pulse_kick::*;

use fast_protocol::DriverConfig;

use crate::operator_config::ConfigDefinition;
use crate::prelude::*;

/// DriverMode is a wrapper around DriverConfig that allows these features:
/// 1. Referencing switches by name instead of index (delay calculating ID until flatted into the IO network)
/// 2. Sensible defaults for every mode, overridable via builders (e.g. `DriverMode::pulse().rest(...).build()`)
/// 3. Hardware values that can be exposed through operator config
#[derive(Clone, Debug, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum DriverMode {
  Pulse(PulseMode),
  PulseKick(PulseKickMode),
  PulseHold(PulseHoldMode),
  PulseHoldCancel(PulseHoldCancelMode),
  DelayedPulse(DelayedPulseMode),
  PulseCancel(PulseCancelMode),
  LongPulse(LongPulseMode),
  PulseHoldExtension(PulseHoldExtensionMode),
  FlipperMainDirect(FlipperMainDirectMode),
  FlipperHoldDirect(FlipperHoldDirectMode),
  FlipperMain3Try(FlipperMain3TryMode),
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

  /// Mode 70 - Pulse for an initial time (up to 255ms), then hold for a secondary time (up to 25s).
  pub fn long_pulse() -> LongPulseModeBuilder {
    LongPulseModeBuilder::default()
  }

  /// Mode 75 - Pulse until the trigger (flip) is deactivated or the cancel switch (flop) is activated.
  pub fn pulse_cancel() -> PulseCancelModeBuilder {
    PulseCancelModeBuilder::default()
  }

  /// Mode 78 - Pulse then hold for a timed period (up to 2.55s), which is reset each time the driver is re-triggered.
  pub fn pulse_hold_extension() -> PulseHoldExtensionModeBuilder {
    PulseHoldExtensionModeBuilder::default()
  }

  /// Mode 93 - Premium flipper driver for hold coil.
  pub fn flipper_hold_direct(button_switch: &'static str) -> FlipperHoldDirectModeBuilder {
    FlipperHoldDirectModeBuilder::new(button_switch)
  }

  /// Mode 94 - Premium flipper driver for main coil.
  pub fn flipper_main_direct(
    button_switch: &'static str,
    eos_switch: &'static str,
  ) -> FlipperMainDirectModeBuilder {
    FlipperMainDirectModeBuilder::new(button_switch, eos_switch)
  }

  /// Mode 95 - Single coil flipper mode
  pub fn flipper_main_3try(
    button_switch: &'static str,
    eos_switch: &'static str,
  ) -> FlipperMain3TryModeBuilder {
    FlipperMain3TryModeBuilder::new(button_switch, eos_switch)
  }

  pub fn to_config(&self, state: &AppState) -> DriverConfig {
    match self {
      Self::Pulse(m) => m.to_config(state),
      Self::PulseKick(m) => m.to_config(state),
      Self::PulseHold(m) => m.to_config(state),
      Self::PulseHoldCancel(m) => m.to_config(state),
      Self::DelayedPulse(m) => m.to_config(state),
      Self::PulseCancel(m) => m.to_config(state),
      Self::LongPulse(m) => m.to_config(state),
      Self::PulseHoldExtension(m) => m.to_config(state),
      Self::FlipperMainDirect(m) => m.to_config(state),
      Self::FlipperHoldDirect(m) => m.to_config(state),
      Self::FlipperMain3Try(m) => m.to_config(state),
    }
  }

  pub fn configs(&self) -> Vec<&ConfigDefinition> {
    match self {
      Self::Pulse(m) => m.configs(),
      Self::PulseKick(m) => m.configs(),
      Self::PulseHold(m) => m.configs(),
      Self::PulseHoldCancel(m) => m.configs(),
      Self::DelayedPulse(m) => m.configs(),
      Self::PulseCancel(m) => m.configs(),
      Self::LongPulse(m) => m.configs(),
      Self::PulseHoldExtension(m) => m.configs(),
      Self::FlipperMainDirect(m) => m.configs(),
      Self::FlipperHoldDirect(m) => m.configs(),
      Self::FlipperMain3Try(m) => m.configs(),
    }
  }

  /// Names of the physical switches this mode references
  pub fn switch_names(&self) -> Vec<&'static str> {
    match self {
      Self::Pulse(m) => m.trigger_mode.switch_names(),
      Self::PulseKick(m) => m.trigger_mode.switch_names(),
      Self::PulseHold(m) => m.trigger_mode.switch_names(),
      Self::PulseHoldCancel(m) => m.trigger_mode.switch_names(),
      Self::DelayedPulse(m) => m.trigger_mode.switch_names(),
      Self::PulseCancel(m) => m.trigger_mode.switch_names(),
      Self::LongPulse(m) => m.trigger_mode.switch_names(),
      Self::PulseHoldExtension(m) => m.trigger_mode.switch_names(),
      Self::FlipperMainDirect(m) => vec![m.button_switch, m.eos_switch],
      Self::FlipperHoldDirect(m) => vec![m.button_switch],
      Self::FlipperMain3Try(m) => vec![m.button_switch, m.eos_switch],
    }
  }

  /// The physical switch that fires the driver, if any. This is the switch a TL sets when activating it
  /// automatically.
  pub fn trigger_switch(&self) -> Option<&'static str> {
    match self {
      Self::Pulse(m) => m.trigger_mode.trigger_switch(),
      Self::PulseKick(m) => m.trigger_mode.trigger_switch(),
      Self::PulseHold(m) => m.trigger_mode.trigger_switch(),
      Self::PulseHoldCancel(m) => m.trigger_mode.trigger_switch(),
      Self::DelayedPulse(m) => m.trigger_mode.trigger_switch(),
      Self::PulseCancel(m) => m.trigger_mode.trigger_switch(),
      Self::LongPulse(m) => m.trigger_mode.trigger_switch(),
      Self::PulseHoldExtension(m) => m.trigger_mode.trigger_switch(),
      Self::FlipperMainDirect(m) => Some(m.button_switch),
      Self::FlipperHoldDirect(m) => Some(m.button_switch),
      Self::FlipperMain3Try(m) => Some(m.button_switch),
    }
  }
}
