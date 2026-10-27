#[derive(Debug, Clone)]
pub enum ActivationMode {
  /// Let the machine decide when to trigger this driver based on its configured trigger
  Automatic,
  /// Manually trigger (fire/activate) the driver immediately
  Tap,
  /// Set virtual switch to 'on' for hold trigger modes
  VirtualSwitchOn,
}

#[derive(Debug, Clone)]
pub enum DeactivationMode {
  Disabled,
  /// set virtual switch to 'off' for hold trigger modes
  VirtualSwitchOff,
}
