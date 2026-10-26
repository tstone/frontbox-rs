use crate::prelude::*;

fn switch_id(name: &'static str, snap: &BootSnapshot) -> Option<usize> {
  snap.switches.by_name(name).map(|sw| sw.id)
}

/// Return the switch ID and invert status from trigger mode
pub fn get_switch_id_and_invert(
  trigger_mode: &DriverTriggerMode,
  snap: &BootSnapshot,
) -> (Option<usize>, Option<bool>) {
  match trigger_mode {
    DriverTriggerMode::Disabled => (None, None),
    DriverTriggerMode::Switch(s) => (switch_id(s, snap), Some(false)),
    DriverTriggerMode::InvertedSwitch(s) => (switch_id(s, snap), Some(true)),
    DriverTriggerMode::VirtualSwitchTrue => (None, Some(false)),
    DriverTriggerMode::VirtualSwitchFalse => (None, Some(true)),
  }
}

/// Return both switch IDs and invert statuses from dual trigger mode
pub fn get_switch_ids_and_inverts(
  trigger_mode: &DriverTriggerDualMode,
  snap: &BootSnapshot,
) -> (Option<usize>, Option<bool>, Option<usize>, Option<bool>) {
  match trigger_mode {
    DriverTriggerDualMode::Disabled => (None, None, None, None),
    DriverTriggerDualMode::FlipSwitchTrue_FlopSwitchTrue {
      flip_switch,
      flop_switch,
    } => (
      switch_id(flip_switch, snap),
      Some(false),
      switch_id(flop_switch, snap),
      Some(false),
    ),
    DriverTriggerDualMode::FlipSwitchFalse_FlopSwitchTrue {
      flip_switch,
      flop_switch,
    } => (
      switch_id(flip_switch, snap),
      Some(true),
      switch_id(flop_switch, snap),
      Some(false),
    ),
    DriverTriggerDualMode::FlipSwitchTrue_FlopSwitchFalse {
      flip_switch,
      flop_switch,
    } => (
      switch_id(flip_switch, snap),
      Some(false),
      switch_id(flop_switch, snap),
      Some(true),
    ),
    DriverTriggerDualMode::FlipSwitchFalse_FlopSwitchFalse {
      flip_switch,
      flop_switch,
    } => (
      switch_id(flip_switch, snap),
      Some(true),
      switch_id(flop_switch, snap),
      Some(true),
    ),
    DriverTriggerDualMode::VirtualFlip_FlopSwitchTrue(virtual_flip) => {
      (None, Some(false), switch_id(virtual_flip, snap), Some(false))
    }
    DriverTriggerDualMode::VirtualFlip_FlopSwitchFalse(virtual_flip) => {
      (None, Some(false), switch_id(virtual_flip, snap), Some(true))
    }
    DriverTriggerDualMode::FlipSwitchTrue_VirtualFlop(virtual_flop) => {
      (switch_id(virtual_flop, snap), Some(false), None, Some(false))
    }
    DriverTriggerDualMode::FlipSwitchFalse_VirtualFlop(virtual_flop) => {
      (switch_id(virtual_flop, snap), Some(true), None, Some(false))
    }
  }
}
