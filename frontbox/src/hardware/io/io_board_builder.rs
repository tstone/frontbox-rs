use core::panic;

use crate::hardware::io::SwitchConfig;
use crate::{
  DriverDefinition, DriverMapping, DriverMode, HardwareTag, NativeIdentity, SwitchDefinition,
  SwitchMapping,
};

#[derive(Default)]
pub struct IoBoardBuilder {
  pub(crate) description: &'static str,
  pub(crate) switch_count: u32,
  pub(crate) driver_count: u32,
  pub(crate) switches: Vec<SwitchDefinition>,
  pub(crate) drivers: Vec<DriverDefinition>,
}

impl IoBoardBuilder {
  pub fn with(self, declaration: impl Into<IoDeclaration>) -> Self {
    match declaration.into() {
      IoDeclaration::Switch {
        name,
        pin,
        config,
        tags,
      } => self.add_switch(name, pin, tags, Some(config)),
      IoDeclaration::Driver {
        name,
        pin,
        config,
        tags,
      } => self.add_driver(name, pin, tags, config),
    }
  }

  pub fn add_switch(
    mut self,
    name: &'static str,
    pin: u16,
    tags: Vec<Box<dyn HardwareTag>>, // TODO
    config: Option<SwitchConfig>,
  ) -> Self {
    if pin >= self.switch_count as u16 {
      panic!(
        "Switch index {} out of bounds for board with {} switches",
        pin, self.switch_count
      );
    }

    self.switches.push(SwitchDefinition {
      id: pin as usize,
      name,
      native: NativeIdentity {
        board_idx: 0, // This will be set later
        pin: pin as usize,
      },
      config,
      tags,
    });

    self
  }

  pub fn add_driver(
    mut self,
    name: &'static str,
    pin: u16,
    tags: Vec<Box<dyn HardwareTag>>, // TODO
    config: Option<Box<dyn DriverMode>>,
  ) -> Self {
    if pin >= self.driver_count as u16 {
      panic!(
        "Driver index {} out of bounds for board with {} drivers",
        pin, self.driver_count
      );
    }

    self.drivers.push(DriverDefinition {
      id: pin as usize,
      name,
      native: NativeIdentity {
        board_idx: 0, // This will be set later
        pin: pin as usize,
      },
      mode: config,
      tags,
    });

    self
  }
}

pub enum IoDeclaration {
  Switch {
    name: &'static str,
    pin: u16,
    config: SwitchConfig,
    tags: Vec<Box<dyn HardwareTag>>,
  },
  Driver {
    name: &'static str,
    pin: u16,
    config: Option<Box<dyn DriverMode>>,
    tags: Vec<Box<dyn HardwareTag>>,
  },
}

impl From<SwitchMapping> for IoDeclaration {
  fn from(mapping: SwitchMapping) -> Self {
    IoDeclaration::Switch {
      name: mapping.key,
      pin: mapping.pin,
      config: mapping.config.unwrap_or_default(),
      tags: mapping.tags,
    }
  }
}

impl From<DriverMapping> for IoDeclaration {
  fn from(mapping: DriverMapping) -> Self {
    IoDeclaration::Driver {
      name: mapping.key,
      pin: mapping.pin,
      config: mapping.mode,
      tags: mapping.tags,
    }
  }
}
