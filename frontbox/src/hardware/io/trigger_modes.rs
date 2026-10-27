/// DriverTriggerMode answers: What causes the driver to fire?
#[derive(Debug, Clone, Default, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum DriverTriggerMode {
  #[default]
  Disabled,
  /// Driver is active when switch is closed
  Switch(&'static str),
  /// Driver is active when switch is open
  InvertedSwitch(&'static str),
  /// Driver is active when virtual switch (manually triggered) is true/on
  VirtualSwitchTrue,
  /// Driver is active when virtual switch (manually triggered) is false/off
  VirtualSwitchFalse,
}

impl DriverTriggerMode {
  /// Names of the physical switches this trigger references
  pub fn switch_names(&self) -> Vec<&'static str> {
    match self {
      Self::Switch(name) | Self::InvertedSwitch(name) => vec![name],
      Self::Disabled | Self::VirtualSwitchTrue | Self::VirtualSwitchFalse => vec![],
    }
  }

  /// The physical switch that fires the driver, if any
  pub fn trigger_switch(&self) -> Option<&'static str> {
    match self {
      Self::Switch(name) | Self::InvertedSwitch(name) => Some(name),
      Self::Disabled | Self::VirtualSwitchTrue | Self::VirtualSwitchFalse => None,
    }
  }
}

/// DriverTriggerDualMode answers: What causes the driver to fire when two switches are involved?
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Default, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum DriverTriggerDualMode {
  #[default]
  Disabled,
  /// Driver is active when flip switch and flop switch are closed
  FlipSwitchTrue_FlopSwitchTrue {
    flip_switch: &'static str,
    flop_switch: &'static str,
  },
  /// Driver is active when flip switch is closed and flop switch is open
  FlipSwitchFalse_FlopSwitchTrue {
    flip_switch: &'static str,
    flop_switch: &'static str,
  },
  /// Driver is active when flip switch is open and flop switch is closed
  FlipSwitchTrue_FlopSwitchFalse {
    flip_switch: &'static str,
    flop_switch: &'static str,
  },
  /// Driver is active when flip switch and flop switch are open
  FlipSwitchFalse_FlopSwitchFalse {
    flip_switch: &'static str,
    flop_switch: &'static str,
  },
  /// Driver is active when virtual switch (manually triggered) is true/on and flop switch is closed
  VirtualFlip_FlopSwitchTrue(&'static str),
  /// Driver is active when virtual switch (manually triggered) is true/on and flop switch is open
  VirtualFlip_FlopSwitchFalse(&'static str),
  /// Driver is active when flip switch is closed and virtual switch (manually triggered) is true/on
  FlipSwitchTrue_VirtualFlop(&'static str),
  /// Driver is active when flip switch is open and virtual switch (manually triggered) is true/on
  FlipSwitchFalse_VirtualFlop(&'static str),
}

impl DriverTriggerDualMode {
  /// Names of the physical switches this trigger references
  pub fn switch_names(&self) -> Vec<&'static str> {
    match self {
      Self::Disabled => vec![],
      Self::FlipSwitchTrue_FlopSwitchTrue {
        flip_switch,
        flop_switch,
      }
      | Self::FlipSwitchFalse_FlopSwitchTrue {
        flip_switch,
        flop_switch,
      }
      | Self::FlipSwitchTrue_FlopSwitchFalse {
        flip_switch,
        flop_switch,
      }
      | Self::FlipSwitchFalse_FlopSwitchFalse {
        flip_switch,
        flop_switch,
      } => vec![flip_switch, flop_switch],
      Self::VirtualFlip_FlopSwitchTrue(name)
      | Self::VirtualFlip_FlopSwitchFalse(name)
      | Self::FlipSwitchTrue_VirtualFlop(name)
      | Self::FlipSwitchFalse_VirtualFlop(name) => vec![name],
    }
  }

  /// The physical flip switch that fires the driver, if any. The flop switch only gates it.
  pub fn trigger_switch(&self) -> Option<&'static str> {
    match self {
      Self::FlipSwitchTrue_FlopSwitchTrue { flip_switch, .. }
      | Self::FlipSwitchFalse_FlopSwitchTrue { flip_switch, .. }
      | Self::FlipSwitchTrue_FlopSwitchFalse { flip_switch, .. }
      | Self::FlipSwitchFalse_FlopSwitchFalse { flip_switch, .. } => Some(flip_switch),
      Self::FlipSwitchTrue_VirtualFlop(name) | Self::FlipSwitchFalse_VirtualFlop(name) => Some(name),
      Self::Disabled | Self::VirtualFlip_FlopSwitchTrue(_) | Self::VirtualFlip_FlopSwitchFalse(_) => {
        None
      }
    }
  }
}
