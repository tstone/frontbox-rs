use std::time::Duration;

use fast_protocol::DriverConfig;

/** This module containes the "final" form that is shared with the rest of the code */

#[derive(Debug, Clone)]
pub struct IoNetwork {
  pub boards: Vec<IoBoardDefinition>,
  pub switches: Vec<SwitchDefinition>,
  pub drivers: Vec<DriverDefinition>,
}

/// Simplified description of an IO board
#[derive(Debug, Clone)]
pub struct IoBoardDefinition {
  pub description: &'static str,
  pub switch_count: u32,
  pub driver_count: u32,
}

#[derive(Debug, Clone)]
pub struct SwitchDefinition {
  pub id: usize,
  pub name: &'static str,
  pub parent_index: u8,
  pub config: Option<SwitchConfig>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct DriverDefinition {
  pub id: usize,
  pub name: &'static str,
  pub parent_index: u8,
  pub config: Option<DriverConfig>,
}
