use core::panic;
use std::collections::HashMap;

use bevy_ecs::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct IoBoardSpec {
  pub switch_count: u32,
  pub driver_count: u32,
  pub switch_map: HashMap<u16, &'static str>,
  pub driver_map: HashMap<u16, &'static str>,
}

impl IoBoardSpec {
  pub fn with_switch(mut self, idx: u16, key: &'static str) -> Self {
    if idx >= self.switch_count as u16 {
      panic!(
        "Switch index {} out of bounds for board with {} switches",
        idx, self.switch_count
      );
    }

    self.switch_map.insert(idx, key);
    self
  }

  pub fn with_driver_pin(mut self, idx: u16, key: &'static str) -> Self {
    if idx >= self.driver_count as u16 {
      panic!(
        "Driver index {} out of bounds for board with {} drivers",
        idx, self.driver_count
      );
    }

    self.driver_map.insert(idx, key);
    self
  }
}

pub struct FastIoBoards;

impl FastIoBoards {
  pub fn custom(switch_count: u32, driver_count: u32) -> IoBoardSpec {
    IoBoardSpec {
      switch_count,
      driver_count,
      switch_map: HashMap::new(),
      driver_map: HashMap::new(),
    }
  }

  pub fn io_3208() -> IoBoardSpec {
    IoBoardSpec {
      switch_count: 32,
      driver_count: 8,
      switch_map: HashMap::new(),
      driver_map: HashMap::new(),
    }
  }

  pub fn io_1616() -> IoBoardSpec {
    IoBoardSpec {
      switch_count: 16,
      driver_count: 16,
      switch_map: HashMap::new(),
      driver_map: HashMap::new(),
    }
  }

  pub fn io_0804() -> IoBoardSpec {
    IoBoardSpec {
      switch_count: 8,
      driver_count: 4,
      switch_map: HashMap::new(),
      driver_map: HashMap::new(),
    }
  }

  pub fn cabinet() -> IoBoardSpec {
    IoBoardSpec {
      switch_count: 24,
      driver_count: 8,
      switch_map: HashMap::new(),
      driver_map: HashMap::new(),
    }
  }
}

#[derive(Component, Debug, Clone)]
pub struct IoBoardDefinition {
  pub index: u8,
  pub switch_offset: u32,
  pub driver_offset: u32,
  pub switch_count: u32,
  pub driver_count: u32,
  pub switch_map: HashMap<u16, &'static str>,
  pub driver_map: HashMap<u16, &'static str>,
}

#[derive(Debug, Clone)]
pub struct IoNetworkResources {
  pub switches: Vec<Switch>,
  pub driver_pins: Vec<DriverPin>,
}

pub struct IoNetworkSpec {
  specs: Vec<IoBoardSpec>,
}

impl IoNetworkSpec {
  pub fn new() -> Self {
    Self { specs: Vec::new() }
  }

  pub fn add_board(&mut self, spec: IoBoardSpec) {
    self.specs.push(spec);
  }

  pub fn build(self) -> IoNetworkResources {
    let mut switches = Vec::new();
    let mut driver_pins = Vec::new();
    let mut switch_offset = 0;
    let mut driver_offset = 0;

    for (i, spec) in self.specs.into_iter().enumerate() {
      for (idx, name) in spec.switch_map.iter() {
        switches.push(Switch {
          id: switch_offset as usize + *idx as usize,
          name: *name,
          parent_index: i as u8,
        });
      }

      for (idx, name) in spec.driver_map.iter() {
        driver_pins.push(DriverPin {
          id: driver_offset as usize + *idx as usize,
          name: *name,
          parent_index: i as u8,
        });
      }

      switch_offset += spec.switch_count;
      driver_offset += spec.driver_count;
    }

    IoNetworkResources {
      switches,
      driver_pins,
    }
  }
}

#[derive(Debug, Clone, Component)]
pub struct Switch {
  pub id: usize,
  pub name: &'static str,
  pub parent_index: u8,
}

#[derive(Debug, Clone, Component)]
pub struct DriverPin {
  pub id: usize,
  pub name: &'static str,
  pub parent_index: u8,
}
