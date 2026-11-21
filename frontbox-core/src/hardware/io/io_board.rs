use crate::prelude::*;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct IoBoard {
  pub description: &'static str,
  pub switch_count: u16,
  pub driver_count: u16,
  pub driver_voltage: DriverVoltage,
}

impl IoBoard {
  pub fn custom(
    switch_count: u16,
    driver_count: u16,
    driver_voltage: DriverVoltage,
  ) -> IoBoardBuilder {
    IoBoardBuilder::new(
      Box::leak(
        format!(
          "Custom IO Board ({} switches, {} drivers)",
          switch_count, driver_count
        )
        .into_boxed_str(),
      ),
      switch_count,
      driver_count,
      driver_voltage,
    )
  }

  /// The voltage of the driver on `pin`. Every driver on a board is the same voltage today, but callers ask per pin so
  /// a board that mixes them changes only this.
  pub fn voltage_at(&self, _pin: u16) -> DriverVoltage {
    self.driver_voltage
  }

  pub fn io_3208() -> IoBoardBuilder {
    IoBoardBuilder::new("IO-3208", 32, 8, DriverVoltage::High)
  }

  pub fn io_1616() -> IoBoardBuilder {
    IoBoardBuilder::new("IO-1616", 16, 16, DriverVoltage::High)
  }

  pub fn io_0804() -> IoBoardBuilder {
    IoBoardBuilder::new("IO-0804", 8, 4, DriverVoltage::High)
  }

  pub fn cabinet() -> IoBoardBuilder {
    IoBoardBuilder::new("Cabinet IO", 24, 8, DriverVoltage::Low)
  }
}
