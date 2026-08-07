use crate::hardware::Switch;

/// Runs when a switch becomes closed (depressed)
#[derive(Debug)]
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
#[derive(Debug)]
#[allow(unused)]
pub struct SwitchOpened {
  pub switch: Switch,
}

impl SwitchOpened {
  pub fn new(switch: Switch) -> SwitchOpened {
    Self { switch }
  }
}