use crate::prelude::*;
use serde::Serialize;

pub struct IoNetwork {
  pub boards: Vec<IoBoard>,
  pub switches: Vec<Addressed<SwitchDefinition>>,
  pub drivers: Vec<DriverDefinition>,
}

pub struct ResolvedIoNetwork {
  pub boards: Vec<ResolvedIoBoard>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct IoBoard {
  pub description: &'static str,
  pub switch_count: u16,
  pub driver_count: u16,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ResolvedIoBoard {
  pub node_id: u8,
  pub name: String,
  pub board_revision: u16,
  pub firmware_version: String,
  pub description: &'static str,
  pub switch_count: u16,
  pub driver_count: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeIdentity {
  pub board_idx: usize,
  pub pin: usize,
}

impl NativeIdentity {
  pub fn new(board_idx: usize, pin: usize) -> Self {
    Self { board_idx, pin }
  }
}
