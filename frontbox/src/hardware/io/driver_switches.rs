use crate::prelude::*;

fn switch_id(name: &'static str, ctx: &ContextBase) -> Option<usize> {
  ctx.switches.by_name(name).map(|sw| sw.id)
}

/// Return the switch ID and invert status from trigger mode
pub fn get_switch_id_and_invert(
  trigger_mode: &DriverTriggerMode,
  ctx: &ContextBase,
) -> (Option<usize>, Option<bool>) {
  match trigger_mode {
    DriverTriggerMode::Disabled => (None, None),
    DriverTriggerMode::Switch(s) => (switch_id(s, ctx), Some(false)),
    DriverTriggerMode::InvertedSwitch(s) => (switch_id(s, ctx), Some(true)),
    DriverTriggerMode::VirtualSwitchTrue => (None, Some(false)),
    DriverTriggerMode::VirtualSwitchFalse => (None, Some(true)),
  }
}

/// Return both switch IDs and invert statuses from dual trigger mode
pub fn get_switch_ids_and_inverts(
  trigger_mode: &DriverTriggerDualMode,
  ctx: &ContextBase,
) -> (Option<usize>, Option<bool>, Option<usize>, Option<bool>) {
  match trigger_mode {
    DriverTriggerDualMode::Disabled => (None, None, None, None),
    DriverTriggerDualMode::FlipSwitchTrue_FlopSwitchTrue {
      flip_switch,
      flop_switch,
    } => (
      switch_id(flip_switch, ctx),
      Some(false),
      switch_id(flop_switch, ctx),
      Some(false),
    ),
    DriverTriggerDualMode::FlipSwitchFalse_FlopSwitchTrue {
      flip_switch,
      flop_switch,
    } => (
      switch_id(flip_switch, ctx),
      Some(true),
      switch_id(flop_switch, ctx),
      Some(false),
    ),
    DriverTriggerDualMode::FlipSwitchTrue_FlopSwitchFalse {
      flip_switch,
      flop_switch,
    } => (
      switch_id(flip_switch, ctx),
      Some(false),
      switch_id(flop_switch, ctx),
      Some(true),
    ),
    DriverTriggerDualMode::FlipSwitchFalse_FlopSwitchFalse {
      flip_switch,
      flop_switch,
    } => (
      switch_id(flip_switch, ctx),
      Some(true),
      switch_id(flop_switch, ctx),
      Some(true),
    ),
    DriverTriggerDualMode::VirtualFlip_FlopSwitchTrue(virtual_flip) => {
      (None, Some(false), switch_id(virtual_flip, ctx), Some(false))
    }
    DriverTriggerDualMode::VirtualFlip_FlopSwitchFalse(virtual_flip) => {
      (None, Some(false), switch_id(virtual_flip, ctx), Some(true))
    }
    DriverTriggerDualMode::FlipSwitchTrue_VirtualFlop(virtual_flop) => {
      (switch_id(virtual_flop, ctx), Some(false), None, Some(false))
    }
    DriverTriggerDualMode::FlipSwitchFalse_VirtualFlop(virtual_flop) => {
      (switch_id(virtual_flop, ctx), Some(true), None, Some(false))
    }
  }
}
