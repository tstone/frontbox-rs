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
