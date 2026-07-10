use core::panic;

use crate::hardware::io::SwitchConfig;
use crate::prelude::{BoardAssignment, SwitchDefinition, Wired};
use crate::{DriverDefinition, DriverMapping, DriverMode, NativeIdentity, Tag};

#[derive(Default)]
pub struct IoBoardBuilder {
  pub(crate) description: &'static str,
  pub(crate) switch_count: u16,
  pub(crate) driver_count: u16,
  pub(crate) switches: Vec<Wired<SwitchDefinition>>,
  pub(crate) drivers: Vec<DriverDefinition>,
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

  pub fn with(self, declaration: impl Into<IoDeclaration>) -> Self {
    match declaration.into() {
      IoDeclaration::Switch { .. } => self,
      IoDeclaration::Driver {
        name,
        pin,
        config,
        tags,
      } => self.add_driver(name, pin, tags, config),
    }
  }

  pub fn add_driver(
    mut self,
    name: &'static str,
    pin: u16,
    tags: Vec<Box<dyn Tag>>, // TODO
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
    tags: Vec<Box<dyn Tag>>,
  },
  Driver {
    name: &'static str,
    pin: u16,
    config: Option<Box<dyn DriverMode>>,
    tags: Vec<Box<dyn Tag>>,
  },
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
