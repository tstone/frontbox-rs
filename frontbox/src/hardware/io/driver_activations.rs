#[derive(Debug, Clone)]
pub enum ActivationMode {
  /// Let the machine decide when to trigger this driver based on its configured trigger
  /// FAST clears out the switch whenever the driver is disabled, so it needs to be re-set
  /// each time it is activated.
  Automatic(&'static str),
  /// Manually trigger (fire/activate) the driver immediately
  Tap,
  /// Set virtual switch to 'on' for hold trigger modes
  VirtualSwitchOn,
}

impl ActivationMode {
  pub fn switch_name(&self) -> Option<&'static str> {
    match self {
      Self::Automatic(name) => Some(name),
      _ => None,
    }
  }
}

#[derive(Debug, Clone)]
pub enum DeactivationMode {
  Disabled,
  /// set virtual switch to 'off' for hold trigger modes
  VirtualSwitchOff,
}
