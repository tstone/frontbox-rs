use std::collections::HashMap;

use fast_protocol::{DriverTriggerControlMode, TriggerDriverCommand};

/// How each driver is triggered on the hardware right now, as far as we know. FAST never reports this, so it's kept
/// from the TLs and DLs sent to each driver.
///
/// FAST disconnects a driver's switch whenever the driver is sent a DL, so the Neuron machine uses this to trigger a
/// driver again the way it was after its configuration is updated. `MachineSystem` uses it to know which drivers the
/// hardware fires on its own when their switch changes.
#[derive(Default)]
pub(crate) struct DriverTriggers {
  /// A driver missing here is `Off`
  triggers: HashMap<usize, DriverTrigger>,
}

impl DriverTriggers {
  pub fn triggered(
    &mut self,
    driver_id: usize,
    mode: DriverTriggerControlMode,
    switch: Option<usize>,
  ) {
    match mode {
      DriverTriggerControlMode::Automatic => {
        self.triggers.insert(driver_id, DriverTrigger::Automatic { switch });
      }
      DriverTriggerControlMode::On => {
        self.triggers.insert(driver_id, DriverTrigger::On);
      }
      DriverTriggerControlMode::Off => {
        self.triggers.remove(&driver_id);
      }
      // a tap is one-shot, so it leaves the driver as it was
      DriverTriggerControlMode::Manual => {}
    }
  }

  /// The driver was given a new configuration, which leaves it disconnected until it's activated again
  pub fn cleared(&mut self, driver_id: usize) {
    self.triggers.remove(&driver_id);
  }

  pub fn current_trigger(&self, driver_id: usize) -> DriverTrigger {
    self.triggers.get(&driver_id).copied().unwrap_or(DriverTrigger::Off)
  }

  /// The TL that puts the driver back the way it was after a DL, unless it was off
  pub fn restore(&self, driver_id: usize) -> Option<TriggerDriverCommand> {
    match self.current_trigger(driver_id) {
      DriverTrigger::Automatic { switch } => Some(TriggerDriverCommand::new(
        driver_id,
        DriverTriggerControlMode::Automatic,
        switch,
      )),
      DriverTrigger::On => Some(TriggerDriverCommand::new(driver_id, DriverTriggerControlMode::On, None)),
      DriverTrigger::Off => None,
    }
  }

  /// Every driver that isn't off, with the mode and switch to trigger it the way it is again
  pub fn live(&self) -> impl Iterator<Item = (usize, DriverTriggerControlMode, Option<usize>)> + '_ {
    self.triggers.iter().filter_map(|(driver_id, trigger)| match trigger {
      DriverTrigger::Automatic { switch } => Some((*driver_id, DriverTriggerControlMode::Automatic, *switch)),
      DriverTrigger::On => Some((*driver_id, DriverTriggerControlMode::On, None)),
      DriverTrigger::Off => None,
    })
  }

  /// Drivers the hardware fires on its own when this switch changes
  pub fn automatic_on(&self, switch_id: usize) -> impl Iterator<Item = usize> + '_ {
    self
      .triggers
      .iter()
      .filter(move |(_, trigger)| **trigger == DriverTrigger::Automatic { switch: Some(switch_id) })
      .map(|(driver_id, _)| *driver_id)
  }
}

/// How a driver is triggered on the hardware
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverTrigger {
  /// Never triggered, turned off, or configured again since
  Off,
  /// The hardware fires it on its own when its switch changes, as its configuration says
  Automatic { switch: Option<usize> },
  /// Held on
  On,
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
  fn an_automatic_driver_is_restored() {
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
  fn a_tap_leaves_an_automatic_driver_automatic() {
    let mut triggers = DriverTriggers::default();
    triggers.triggered(COIL, DriverTriggerControlMode::Automatic, Some(0x12));
    triggers.triggered(COIL, DriverTriggerControlMode::Manual, None);
    assert_eq!(restored(&triggers).as_deref(), Some("TL:A,0,12\r"));
  }

  #[test]
  fn only_automatic_drivers_fire_on_their_switch() {
    let mut triggers = DriverTriggers::default();
    triggers.triggered(COIL, DriverTriggerControlMode::Automatic, Some(0x12));
    triggers.triggered(0xB, DriverTriggerControlMode::On, None);
    triggers.triggered(0xC, DriverTriggerControlMode::Automatic, Some(0x13));
    assert_eq!(triggers.automatic_on(0x12).collect::<Vec<_>>(), vec![COIL]);
  }

  #[test]
  fn a_new_configuration_clears_it() {
    let mut triggers = DriverTriggers::default();
    triggers.triggered(COIL, DriverTriggerControlMode::Automatic, Some(0x12));
    triggers.cleared(COIL);
    assert_eq!(restored(&triggers), None);
  }
}
