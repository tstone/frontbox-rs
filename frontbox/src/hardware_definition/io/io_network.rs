use std::collections::HashMap;
use std::time::Duration;

use crate::prelude::*;
use fast_protocol::DriverConfig;
use serde::Serialize;

/** This module containes the "final" form that is shared with the rest of the code */

#[derive(Debug)]
pub struct IoNetwork {
  pub boards: Vec<IoBoard>,
  pub switches: Vec<SwitchDefinition>,
  pub drivers: Vec<Driver>,
  pub driver_groups: HashMap<&'static str, Vec<&'static str>>,
}

pub type DriverGroups = StorableHashMap<&'static str, Vec<&'static str>>;

#[derive(Debug, Clone, Serialize, Storable, Hash, PartialEq, Eq)]
pub struct IoBoard {
  pub description: &'static str,
  pub switch_count: u32,
  pub driver_count: u32,
}

pub type IoBoards = StorableHashSet<IoBoard>;

#[derive(Debug, Clone)]
pub struct SwitchDefinition {
  pub id: usize,
  pub name: &'static str,
  pub native: NativeIdentity,
  pub config: Option<SwitchConfig>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeIdentity {
  pub board_idx: usize,
  pub pin: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SwitchConfig {
  pub inverted: bool,
  pub debounce_close: Option<Duration>,
  pub debounce_open: Option<Duration>,
}

impl Default for SwitchConfig {
  fn default() -> Self {
    Self {
      inverted: false,
      debounce_close: None,
      debounce_open: None,
    }
  }
}
