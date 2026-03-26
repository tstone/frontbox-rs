use crate::prelude::*;
use serde::Serialize;

/** This module contains the "final" form that is shared with the rest of the code */

pub struct IoNetwork {
  pub boards: Vec<IoBoard>,
  pub switches: Vec<SwitchDefinition>,
  pub drivers: Vec<DriverDefinition>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct IoBoard {
  pub description: &'static str,
  pub switch_count: u32,
  pub driver_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeIdentity {
  pub board_idx: usize,
  pub pin: usize,
}
