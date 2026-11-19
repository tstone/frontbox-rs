use crate::prelude::*;

pub struct IoBoardBuilder {
  pub(crate) description: &'static str,
  pub(crate) switch_count: u16,
  pub(crate) driver_count: u16,
  pub(crate) driver_voltage: DriverVoltage,
  pub(crate) switches: Vec<IoWired<SwitchDefinition>>,
  pub(crate) drivers: Vec<IoWired<DriverDefinition>>,
}

impl IoBoardBuilder {
  pub(crate) fn new(
    description: &'static str,
    switch_count: u16,
    driver_count: u16,
    driver_voltage: DriverVoltage,
  ) -> Self {
    Self {
      description,
      switch_count,
      driver_count,
      driver_voltage,
      switches: Vec::new(),
      drivers: Vec::new(),
    }
  }

  pub fn wire_switch(mut self, pin: u16, switch: &'static SwitchDefinition) -> Self {
    let wired = IoWired::new(
      switch,
      IoAddress {
        board_idx: 0, // Set later
        pin,
      },
    );
    self.switches.push(wired);
    self
  }

  pub fn wire_driver(mut self, pin: u16, driver: &'static DriverDefinition) -> Self {
    let wired = IoWired::new(driver, IoAddress::new(0, pin));
    self.drivers.push(wired);
    self
  }
}
