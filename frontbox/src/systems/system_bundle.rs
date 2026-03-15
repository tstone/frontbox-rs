use crate::machine::watchdog::Watchdog;
use crate::prelude::System;

pub mod bundles {
  use super::*;

  pub fn minimal() -> Vec<Box<dyn System>> {
    vec![Watchdog::new()]
  }
}
