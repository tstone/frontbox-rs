use crate::{DriverLookup, ExpansionBoard, IoBoard, SwitchLookup};

pub struct Hardware {
  pub switches: SwitchLookup,
  pub drivers: DriverLookup,
  pub io_network: Vec<IoBoard>,
  pub exp_network: Vec<ExpansionBoard>,
}

impl Hardware {
  pub fn new(
    switches: SwitchLookup,
    drivers: DriverLookup,
    io_network: Vec<IoBoard>,
    exp_network: Vec<ExpansionBoard>,
  ) -> Self {
    Self {
      switches,
      drivers,
      io_network,
      exp_network,
    }
  }
}
