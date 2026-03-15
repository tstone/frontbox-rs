use crate::DriverMode;

pub struct WatchdogPing;
pub struct ClearWatchdog;
pub struct ResetExpansionNetwork;

pub struct ConfigureDriver {
  pub driver: &'static str,
  pub mode: Box<dyn DriverMode>,
}

impl ConfigureDriver {
  pub fn new(driver: &'static str, mode: impl DriverMode + 'static) -> Self {
    Self {
      driver,
      mode: Box::new(mode),
    }
  }
}

pub struct ActivateDriver {
  pub driver: &'static str,
  pub mode: ActivationMode,
}

impl ActivateDriver {
  pub fn new(driver: &'static str, mode: ActivationMode) -> Self {
    Self { driver, mode }
  }
}

pub struct DeactivateDriver {
  pub driver: &'static str,
  pub mode: DeactivationMode,
}

impl DeactivateDriver {
  pub fn new(driver: &'static str, mode: DeactivationMode) -> Self {
    Self { driver, mode }
  }
}

pub struct ActivateDriverGroup {
  pub group: &'static str,
  pub mode: ActivationMode,
}

impl ActivateDriverGroup {
  pub fn new(group: &'static str, mode: ActivationMode) -> Self {
    Self { group, mode }
  }
}

pub struct DeactivateDriverGroup {
  pub group: &'static str,
  pub mode: DeactivationMode,
}

impl DeactivateDriverGroup {
  pub fn new(group: &'static str, mode: DeactivationMode) -> Self {
    Self { group, mode }
  }
}

#[derive(Debug, Clone)]
pub enum ActivationMode {
  /// let the machine decide when to trigger this driver based on its configured trigger
  Automatic,
  /// manually trigger (activate) the driver immediately
  Tap,
  /// set virtual switch to 'on' for hold trigger modes
  VirtualSwitchOn,
}

#[derive(Debug, Clone)]
pub enum DeactivationMode {
  Disabled,
  /// set virtual switch to 'off' for hold trigger modes
  VirtualSwitchOff,
}
