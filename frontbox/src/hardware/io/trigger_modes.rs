/// DriverTriggerMode answers: What causes the driver to fire?
#[derive(Debug, Clone, Default)]
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

/// DriverTriggerDualMode answers: What causes the driver to fire when two switches are involved?
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Default)]
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
