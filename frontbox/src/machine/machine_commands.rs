use crate::DriverMode;

pub struct WatchdogPing;
pub struct ClearWatchdog;

/// Reset all expansion boards. This will clear out LEDs, reset servos, etc.
pub struct ResetExpansionNetwork;

/// This performs a "report switches" action against FAST hardware and updates SwitchLookup
pub struct RefreshSwitchState;

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

#[derive(Debug, Clone)]
pub enum ActivationMode {
  /// Let the machine decide when to trigger this driver based on its configured trigger
  /// FAST clears out the switch whenever the driver is disabled, so it needs to be re-set
  /// each time it is activated.
  Automatic(&'static str),
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
