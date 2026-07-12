use crate::DriverDefinition;
use crate::prelude::{BoardAssignment, SwitchDefinition, Wired};

#[derive(Default)]
pub struct IoBoardBuilder {
  pub(crate) description: &'static str,
  pub(crate) switch_count: u16,
  pub(crate) driver_count: u16,
  pub(crate) switches: Vec<Wired<SwitchDefinition>>,
  pub(crate) drivers: Vec<Wired<DriverDefinition>>,
}

impl IoBoardBuilder {
  pub fn wire_switch(mut self, pin: u16, switch: &SwitchDefinition) -> Self {
    let wired = Wired::new(
      switch.clone(),
      BoardAssignment::IO {
        board_idx: 0, // Set later
        pin,
      },
    );
    self.switches.push(wired);
    self
  }

  pub fn wire_driver(mut self, pin: u16, driver: &DriverDefinition) -> Self {
    let wired = Wired::new(driver.clone(), BoardAssignment::IO { board_idx: 0, pin });
    self.drivers.push(wired);
    self
  }
}
