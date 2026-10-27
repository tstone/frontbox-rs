use std::collections::HashMap;

use fast_protocol::{DriverTriggerControlMode, TriggerDriverCommand};

/// FAST disconnects a driver's switch whenever the driver is sent a DL, so a driver that was live has to be triggered
/// again after its configuration is updated. This keeps the last TL that left each driver live: automatic on a
/// switch, or held on.
#[derive(Default)]
pub(super) struct DriverTriggers {
  standing: HashMap<usize, (DriverTriggerControlMode, Option<usize>)>,
}

impl DriverTriggers {
  pub fn triggered(
    &mut self,
    driver_id: usize,
    mode: DriverTriggerControlMode,
    switch: Option<usize>,
  ) {
    match mode {
      DriverTriggerControlMode::Automatic | DriverTriggerControlMode::On => {
        self.standing.insert(driver_id, (mode, switch));
      }
      DriverTriggerControlMode::Off => {
        self.standing.remove(&driver_id);
      }
      // a tap is one-shot, so it leaves the driver as it was
      DriverTriggerControlMode::Manual => {}
    }
  }

  /// The driver was given a new configuration, which leaves it disconnected until it's activated again
  pub fn cleared(&mut self, driver_id: usize) {
    self.standing.remove(&driver_id);
  }

  /// The TL that puts the driver back the way it was after a DL, if it was live
  pub fn restore(&self, driver_id: usize) -> Option<TriggerDriverCommand> {
    self
      .standing
      .get(&driver_id)
      .map(|(mode, switch)| TriggerDriverCommand::new(driver_id, *mode, *switch))
  }
}

#[cfg(test)]
mod tests {
  use fast_protocol::FastBinaryDispatch;

  use super::*;

  const COIL: usize = 0xA;

  fn restored(triggers: &DriverTriggers) -> Option<String> {
    triggers
      .restore(COIL)
      .map(|command| String::from_utf8(command.to_bytes()).unwrap())
  }

  #[test]
  fn a_live_driver_is_restored() {
    let mut triggers = DriverTriggers::default();
    triggers.triggered(COIL, DriverTriggerControlMode::Automatic, Some(0x12));
    assert_eq!(restored(&triggers).as_deref(), Some("TL:A,0,12\r"));
  }

  #[test]
  fn a_held_driver_is_restored() {
    let mut triggers = DriverTriggers::default();
    triggers.triggered(COIL, DriverTriggerControlMode::On, None);
    assert_eq!(restored(&triggers).as_deref(), Some("TL:A,3,\r"));
  }

  #[test]
  fn a_disabled_driver_stays_disabled() {
    let mut triggers = DriverTriggers::default();
    triggers.triggered(COIL, DriverTriggerControlMode::Automatic, Some(0x12));
    triggers.triggered(COIL, DriverTriggerControlMode::Off, None);
    assert_eq!(restored(&triggers), None);
  }

  #[test]
  fn a_tap_is_not_replayed() {
    let mut triggers = DriverTriggers::default();
    triggers.triggered(COIL, DriverTriggerControlMode::Manual, None);
    assert_eq!(restored(&triggers), None);
  }

  #[test]
  fn a_tap_leaves_a_live_driver_live() {
    let mut triggers = DriverTriggers::default();
    triggers.triggered(COIL, DriverTriggerControlMode::Automatic, Some(0x12));
    triggers.triggered(COIL, DriverTriggerControlMode::Manual, None);
    assert_eq!(restored(&triggers).as_deref(), Some("TL:A,0,12\r"));
  }

  #[test]
  fn a_new_configuration_clears_it() {
    let mut triggers = DriverTriggers::default();
    triggers.triggered(COIL, DriverTriggerControlMode::Automatic, Some(0x12));
    triggers.cleared(COIL);
    assert_eq!(restored(&triggers), None);
  }
}
