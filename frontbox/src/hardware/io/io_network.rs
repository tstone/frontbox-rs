use crate::prelude::*;

/// The I/O network is defined using the `IoNetworkBuilder`. See [Defining Hardware]() guide for more details. I/O network devices can either associate a name with a pin, or can optionally provide a configuration. Configurations given here are automatically applied at startup.
/// 
/// Hardware is defined on a board by specifying it's pin, `switch(3)` and giving it a name `switch(3).named("foo")`. It is a good idea to declare names as constants, wrapped in a module for easy access.
/// 
/// Hardware can also be tagged `.tag(Playfield)`. This serves to _classify_ something about the switch, possibly location or purpose. This makes it easy to implement modes that need to say things like "if any playfield switch has been hit, then...". These tags are arbitrary. Frontbox comes with several, but they can be user-defined as well.
/// 
/// Lastly, depending on the type of hardware being defined, an optional config (`.config(...)`) or mode (`.mode(...)`) can be given.
#[derive(Default)]
pub struct IoNetwork {
  pub boards: Vec<IoBoard>,
  pub switches: Vec<IoAddressed<SwitchDefinition>>,
  pub drivers: Vec<IoAddressed<DriverDefinition>>,
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
