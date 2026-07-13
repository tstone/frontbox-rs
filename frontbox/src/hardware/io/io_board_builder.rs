use crate::DriverDefinition;
use crate::prelude::{IoAddress, IoWired, SwitchDefinition};

#[derive(Default)]
pub struct IoBoardBuilder {
  pub(crate) description: &'static str,
  pub(crate) switch_count: u16,
  pub(crate) driver_count: u16,
  pub(crate) switches: Vec<IoWired<SwitchDefinition>>,
  pub(crate) drivers: Vec<IoWired<DriverDefinition>>,
}

impl IoBoardBuilder {
  pub fn wire_switch(mut self, pin: u16, switch: &SwitchDefinition) -> Self {
    let wired = IoWired::new(
      switch.clone(),
      IoAddress {
        board_idx: 0, // Set later
        pin,
      },
    );
    self.switches.push(wired);
    self
  }

  pub fn wire_driver(mut self, pin: u16, driver: &DriverDefinition) -> Self {
    let wired = IoWired::new(driver.clone(), IoAddress::new(0, pin));
    self.drivers.push(wired);
    self
  }
}
