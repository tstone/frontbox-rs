use crate::prelude::*;

pub struct IoNetwork {
  pub boards: Vec<IoBoard>,
  pub switches: Vec<Addressed<SwitchDefinition>>,
  pub drivers: Vec<Addressed<DriverDefinition>>,
}

impl IoNetwork {
  pub fn new(boards: Vec<IoBoardBuilder>) -> Self {
    let builder = IoNetworkBuilder { boards };
    builder.build()
  }

  pub fn empty() -> Self {
    Self::new(Vec::new())
  }
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
