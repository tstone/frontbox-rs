use std::collections::HashMap;

use fast_protocol::{DriverConfig, DriverTriggerControlMode, SwitchState};

use crate::app::app_message::AppMessage;
use crate::machine::driver_triggers::DriverTriggers;
use crate::machine::{DriverTrigger, MachineMessage};
use crate::prelude::*;

/// The state of a virtual machine's hardware: its switches, how each driver is configured and triggered, and the
/// watchdog. It's kept from the messages `MachineSystem` sends the machine, following the rules FAST hardware does, so
/// what it shows is what reached the hardware rather than what a system asked for.
///
/// It has no channels or tasks. The virtual machine applies each message it receives and forwards the replies to the
/// app, and a test can do the same without tokio.
pub struct VirtualHardware {
  switches: HashMap<usize, VirtualSwitch>,
  switch_ids: HashMap<&'static str, usize>,
  /// Every switch slot on the IO network, wired or not, since a switch report covers them all
  switch_slots: usize,
  drivers: HashMap<usize, DriverRecord>,
  driver_ids: HashMap<&'static str, usize>,
  triggers: DriverTriggers,
  watchdog_enabled: bool,
}

impl VirtualHardware {
  /// Hardware wired as the network describes, with every switch open, no driver configured, and the watchdog off
  pub fn new(io_network: &IoNetwork) -> Self {
    let switches: HashMap<usize, VirtualSwitch> = io_network
      .switches
      .iter()
      .map(|switch| {
        let inverted = switch.definition.config.as_ref().is_some_and(|config| config.inverted);
        (switch.id, VirtualSwitch { inverted, closed: false })
      })
      .collect();
    let wired_slots = switches.keys().max().map_or(0, |id| id + 1);
    let board_slots = io_network.boards.iter().map(|board| board.switch_count as usize).sum();

    Self {
      switch_ids: io_network.switches.iter().map(|s| (s.definition.name, s.id)).collect(),
      switches,
      switch_slots: wired_slots.max(board_slots),
      drivers: io_network.drivers.iter().map(|d| (d.id, DriverRecord::default())).collect(),
      driver_ids: io_network.drivers.iter().map(|d| (d.definition.name, d.id)).collect(),
      triggers: DriverTriggers::default(),
      watchdog_enabled: false,
    }
  }

  pub fn switch_id(&self, name: &str) -> Option<usize> {
    self.switch_ids.get(name).copied()
  }

  /// Whether the switch is closed, as the app sees it once any inversion is applied. `None` if there's no such switch.
  pub fn is_closed(&self, name: &str) -> Option<bool> {
    let id = self.switch_id(name)?;
    self.switches.get(&id).map(|switch| switch.closed)
  }

  /// The driver's state, or `None` if there's no such driver
  pub fn driver(&self, name: &str) -> Option<VirtualDriver> {
    let id = *self.driver_ids.get(name)?;
    let record = self.drivers.get(&id)?;
    Some(VirtualDriver {
      config: record.config.clone(),
      trigger: self.triggers.current_trigger(id),
      taps: record.taps,
    })
  }

  /// Whether the watchdog is running, so the hardware powers down drivers if it isn't fed
  pub fn watchdog_enabled(&self) -> bool {
    self.watchdog_enabled
  }

  /// Every switch's state as FAST reports it (SA): one per slot, before inversion
  pub fn switch_report(&self) -> Vec<SwitchState> {
    (0..self.switch_slots)
      .map(|id| {
        let raw_closed = self.switches.get(&id).is_some_and(|switch| switch.closed != switch.inverted);
        if raw_closed { SwitchState::Closed } else { SwitchState::Open }
      })
      .collect()
  }

  /// Configure every driver from its mode, as the Neuron does before the app starts
  pub(crate) fn configure_drivers(&mut self, state: &AppState) {
    for driver in state.drivers.values() {
      if let Some(mode) = state.drivers.mode(driver.name) {
        self.apply(&MachineMessage::ConfigureDriver {
          driver_id: driver.id,
          config: mode.to_config(state),
        });
      }
    }
  }

  /// Apply a message as the hardware would, returning what the hardware sends back to the app
  pub(crate) fn apply(&mut self, message: &MachineMessage) -> Vec<AppMessage> {
    match message {
      MachineMessage::WatchdogPing => self.watchdog_enabled = true,
      MachineMessage::WatchdogClear => self.watchdog_enabled = false,
      MachineMessage::RefreshSwitchState => return vec![AppMessage::SyncSwitchStates(self.switch_report())],
      MachineMessage::ConfigureDriver { driver_id, config } => {
        // a DL disconnects the driver from how it was triggered
        self.triggers.cleared(*driver_id);
        self.drivers.entry(*driver_id).or_default().config = Some(config.clone());
      }
      MachineMessage::UpdateDriverConfig { driver_id, config } => {
        // sent as a DL, but the machine triggers the driver again the way it was straight after
        self.drivers.entry(*driver_id).or_default().config = Some(config.clone());
      }
      MachineMessage::TriggerDriver { driver_id, mode, switch } => {
        self.triggers.triggered(*driver_id, *mode, *switch);
        let record = self.drivers.entry(*driver_id).or_default();
        if matches!(mode, DriverTriggerControlMode::Manual) && record.can_fire() {
          record.taps += 1;
        }
      }
      // LEDs, servos and board resets aren't modelled yet
      MachineMessage::Dispatch { .. } | MachineMessage::Command { .. } => {}
    }
    Vec::new()
  }

  /// Change a switch, as a ball or a player would, returning the event the hardware reports to the app. Nothing is
  /// reported if the switch is unknown or already in that state.
  #[cfg(any(test, feature = "testing"))]
  pub(crate) fn set_switch(&mut self, switch_id: usize, state: SwitchState) -> Option<AppMessage> {
    let switch = self.switches.get_mut(&switch_id)?;
    let closed = matches!(state, SwitchState::Closed);
    if switch.closed == closed {
      return None;
    }
    switch.closed = closed;
    Some(AppMessage::SwitchStateChange(switch_id, state))
  }
}

/// A driver's state on the virtual hardware
#[derive(Debug, Clone)]
pub struct VirtualDriver {
  /// The configuration last sent to it, if any
  pub config: Option<DriverConfig>,
  pub trigger: DriverTrigger,
  /// How many times it fired from a tap. Taps sent before it was configured, or while it's disabled, don't count,
  /// since the hardware ignores them.
  pub taps: u32,
}

struct VirtualSwitch {
  inverted: bool,
  /// As the app sees it, after inversion
  closed: bool,
}

#[derive(Default)]
struct DriverRecord {
  config: Option<DriverConfig>,
  taps: u32,
}

impl DriverRecord {
  fn can_fire(&self) -> bool {
    !matches!(self.config, None | Some(DriverConfig::Disabled))
  }
}

#[cfg(test)]
mod tests {
  use std::sync::LazyLock;
  use std::time::Duration;

  use fast_protocol::Power;

  use super::*;

  static START: LazyLock<SwitchDefinition> = LazyLock::new(|| SwitchDefinition::new("start").build());
  static TROUGH: LazyLock<SwitchDefinition> = LazyLock::new(|| SwitchDefinition::new("trough").inverted().build());
  static EJECT: LazyLock<DriverDefinition> = LazyLock::new(|| DriverDefinition::new("eject").build());

  const START_ID: usize = 0;
  const TROUGH_ID: usize = 3;
  const EJECT_ID: usize = 0;

  fn hardware() -> VirtualHardware {
    VirtualHardware::new(&IoNetwork::new(vec![
      IoBoard::custom(8, 8)
        .wire_switch(START_ID as u16, &START)
        .wire_switch(TROUGH_ID as u16, &TROUGH)
        .wire_driver(EJECT_ID as u16, &EJECT),
    ]))
  }

  fn pulse() -> DriverConfig {
    DriverConfig::Pulse {
      switch: None,
      invert_switch: None,
      initial_pwm_length: Duration::from_millis(20),
      initial_pwm_power: Power::FULL,
      secondary_pwm_length: Duration::ZERO,
      secondary_pwm_power: Power::FULL,
      rest: Duration::ZERO,
    }
  }

  fn configure(hw: &mut VirtualHardware, config: DriverConfig) {
    hw.apply(&MachineMessage::ConfigureDriver { driver_id: EJECT_ID, config });
  }

  fn trigger(hw: &mut VirtualHardware, mode: DriverTriggerControlMode, switch: Option<usize>) {
    hw.apply(&MachineMessage::TriggerDriver { driver_id: EJECT_ID, mode, switch });
  }

  fn eject(hw: &VirtualHardware) -> VirtualDriver {
    hw.driver("eject").unwrap()
  }

  #[test]
  fn it_starts_open_unconfigured_and_without_a_watchdog() {
    let hw = hardware();
    assert_eq!(hw.is_closed("start"), Some(false));
    assert_eq!(hw.is_closed("trough"), Some(false));
    assert!(eject(&hw).config.is_none());
    assert_eq!(eject(&hw).trigger, DriverTrigger::Off);
    assert!(!hw.watchdog_enabled());
  }

  #[test]
  fn unknown_names_are_none() {
    let hw = hardware();
    assert_eq!(hw.is_closed("nope"), None);
    assert!(hw.driver("nope").is_none());
  }

  #[test]
  fn a_switch_change_is_reported_once() {
    let mut hw = hardware();
    assert!(matches!(
      hw.set_switch(START_ID, SwitchState::Closed),
      Some(AppMessage::SwitchStateChange(START_ID, SwitchState::Closed))
    ));
    assert!(hw.set_switch(START_ID, SwitchState::Closed).is_none());
    assert_eq!(hw.is_closed("start"), Some(true));
  }

  #[test]
  fn an_unknown_switch_changes_nothing() {
    let mut hw = hardware();
    assert!(hw.set_switch(5, SwitchState::Closed).is_none());
    assert_eq!(hw.switch_report()[5], SwitchState::Open);
  }

  #[test]
  fn a_report_covers_every_slot_before_inversion() {
    let mut hw = hardware();
    hw.set_switch(START_ID, SwitchState::Closed);
    let report = hw.switch_report();
    assert_eq!(report.len(), 8);
    assert_eq!(report[START_ID], SwitchState::Closed);
    // open as the app sees it, so closed as the hardware reads it
    assert_eq!(report[TROUGH_ID], SwitchState::Closed);
    hw.set_switch(TROUGH_ID, SwitchState::Closed);
    assert_eq!(hw.switch_report()[TROUGH_ID], SwitchState::Open);
  }

  #[test]
  fn a_refresh_is_answered_with_a_report() {
    let mut hw = hardware();
    hw.set_switch(START_ID, SwitchState::Closed);
    let replies = hw.apply(&MachineMessage::RefreshSwitchState);
    let [AppMessage::SyncSwitchStates(states)] = replies.as_slice() else {
      panic!("expected one switch report");
    };
    assert_eq!(states, &hw.switch_report());
  }

  #[test]
  fn a_report_read_by_the_app_matches_the_hardware() {
    let mut hw = hardware();
    hw.set_switch(TROUGH_ID, SwitchState::Closed);
    let mut lookup = SwitchLookup::new(
      vec![
        IoAddressed { definition: &*START, assignment: IoAddress::new(0, 0), id: START_ID },
        IoAddressed { definition: &*TROUGH, assignment: IoAddress::new(0, 3), id: TROUGH_ID },
      ],
      Vec::new(),
    );
    lookup.update_switch_states(hw.switch_report());
    assert_eq!(lookup.is_closed("start"), Some(false));
    assert_eq!(lookup.is_closed("trough"), Some(true));
  }

  #[test]
  fn configuring_a_driver_leaves_it_off_until_triggered() {
    let mut hw = hardware();
    configure(&mut hw, pulse());
    assert!(matches!(eject(&hw).config, Some(DriverConfig::Pulse { .. })));
    assert_eq!(eject(&hw).trigger, DriverTrigger::Off);

    trigger(&mut hw, DriverTriggerControlMode::Automatic, Some(START_ID));
    assert_eq!(eject(&hw).trigger, DriverTrigger::Automatic { switch: Some(START_ID) });
  }

  #[test]
  fn configuring_an_automatic_driver_again_turns_it_off() {
    let mut hw = hardware();
    configure(&mut hw, pulse());
    trigger(&mut hw, DriverTriggerControlMode::Automatic, Some(START_ID));
    configure(&mut hw, pulse());
    assert_eq!(eject(&hw).trigger, DriverTrigger::Off);
  }

  #[test]
  fn updating_a_drivers_config_keeps_its_trigger() {
    let mut hw = hardware();
    configure(&mut hw, pulse());
    trigger(&mut hw, DriverTriggerControlMode::Automatic, Some(START_ID));
    hw.apply(&MachineMessage::UpdateDriverConfig { driver_id: EJECT_ID, config: DriverConfig::Disabled });
    assert!(matches!(eject(&hw).config, Some(DriverConfig::Disabled)));
    assert_eq!(eject(&hw).trigger, DriverTrigger::Automatic { switch: Some(START_ID) });
  }

  #[test]
  fn a_held_driver_is_on_until_turned_off() {
    let mut hw = hardware();
    configure(&mut hw, pulse());
    trigger(&mut hw, DriverTriggerControlMode::On, None);
    assert_eq!(eject(&hw).trigger, DriverTrigger::On);
    trigger(&mut hw, DriverTriggerControlMode::Off, None);
    assert_eq!(eject(&hw).trigger, DriverTrigger::Off);
  }

  #[test]
  fn a_tap_fires_only_a_configured_driver() {
    let mut hw = hardware();
    trigger(&mut hw, DriverTriggerControlMode::Manual, None);
    assert_eq!(eject(&hw).taps, 0, "not configured yet");

    configure(&mut hw, DriverConfig::Disabled);
    trigger(&mut hw, DriverTriggerControlMode::Manual, None);
    assert_eq!(eject(&hw).taps, 0, "disabled");

    configure(&mut hw, pulse());
    trigger(&mut hw, DriverTriggerControlMode::Manual, None);
    trigger(&mut hw, DriverTriggerControlMode::Manual, None);
    assert_eq!(eject(&hw).taps, 2);
  }

  #[test]
  fn a_tap_leaves_an_automatic_driver_automatic() {
    let mut hw = hardware();
    configure(&mut hw, pulse());
    trigger(&mut hw, DriverTriggerControlMode::Automatic, Some(START_ID));
    trigger(&mut hw, DriverTriggerControlMode::Manual, None);
    assert_eq!(eject(&hw).trigger, DriverTrigger::Automatic { switch: Some(START_ID) });
  }

  #[test]
  fn the_watchdog_runs_from_a_ping_until_cleared() {
    let mut hw = hardware();
    hw.apply(&MachineMessage::WatchdogPing);
    assert!(hw.watchdog_enabled());
    hw.apply(&MachineMessage::WatchdogClear);
    assert!(!hw.watchdog_enabled());
  }
}
