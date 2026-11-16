use crate::hardware::Switch;
use crate::prelude::*;

/// Runs when a switch becomes closed (depressed)
#[derive(serde::Serialize, Event, Debug, Clone)]
#[allow(unused)]
pub struct SwitchClosed {
  pub switch: Switch,
}

impl SwitchClosed {
  pub fn new(switch: Switch) -> SwitchClosed {
    Self { switch }
  }
}

/// Runs when a switch becomes open (released)
#[derive(serde::Serialize, Event, Debug, Clone)]
#[allow(unused)]
pub struct SwitchOpened {
  pub switch: Switch,
}

impl SwitchOpened {
  pub fn new(switch: Switch) -> SwitchOpened {
    Self { switch }
  }
}

/// Emitted once at boot when any switch isn't in its initial state (`SwitchDefinition::initial_state`), such as a
/// stuck flipper button or a coin door left open. Not emitted again after a [`MachineReconnected`].
#[derive(serde::Serialize, Event, Debug, Clone)]
pub struct BootSwitchMisalignmentFound {
  pub switches: Vec<MisalignedSwitch>,
}

/// A switch found at boot in a different state from its initial state
#[derive(serde::Serialize, Debug, Clone)]
pub struct MisalignedSwitch {
  pub switch: Switch,
  /// Its initial state, which it wasn't in
  pub expected: SwitchState,
}

/// Emitted when the connection to the machine's hardware is lost
#[derive(serde::Serialize, Event, Debug, Clone)]
pub struct MachineDisconnected;

/// Emitted when the connection to the machine's hardware comes back after a [`MachineDisconnected`].
#[derive(serde::Serialize, Event, Debug, Clone)]
pub struct MachineReconnected;
