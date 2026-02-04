use core::panic;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct IoBoardSpec {
  pub(crate) switch_count: u32,
  pub(crate) driver_count: u32,
  pub(crate) switch_map: HashMap<u16, &'static str>,
  pub(crate) driver_map: HashMap<u16, &'static str>,
  pub(crate) switch_configs: HashMap<&'static str, SwitchConfig>,
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

  pub fn with_switch_config(mut self, key: &'static str, config: SwitchConfig) -> Self {
    self.switch_configs.insert(key, config);
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
      switch_configs: HashMap::new(),
    }
  }

  pub fn io_3208() -> IoBoardSpec {
    IoBoardSpec {
      switch_count: 32,
      driver_count: 8,
      switch_map: HashMap::new(),
      driver_map: HashMap::new(),
      switch_configs: HashMap::new(),
    }
  }

  pub fn io_1616() -> IoBoardSpec {
    IoBoardSpec {
      switch_count: 16,
      driver_count: 16,
      switch_map: HashMap::new(),
      driver_map: HashMap::new(),
      switch_configs: HashMap::new(),
    }
  }

  pub fn io_0804() -> IoBoardSpec {
    IoBoardSpec {
      switch_count: 8,
      driver_count: 4,
      switch_map: HashMap::new(),
      driver_map: HashMap::new(),
      switch_configs: HashMap::new(),
    }
  }

  pub fn cabinet() -> IoBoardSpec {
    IoBoardSpec {
      switch_count: 24,
      driver_count: 8,
      switch_map: HashMap::new(),
      driver_map: HashMap::new(),
      switch_configs: HashMap::new(),
    }
  }
}

#[derive(Debug, Clone)]
pub struct IoBoardDefinition {
  pub index: u8,
  pub switch_offset: u32,
  pub driver_offset: u32,
  pub switch_count: u32,
  pub driver_count: u32,
  pub switch_map: HashMap<u16, &'static str>,
  pub driver_map: HashMap<u16, &'static str>,
  pub switch_configs: HashMap<&'static str, (bool, Option<Duration>, Option<Duration>)>,
}

#[derive(Debug, Clone)]
pub struct IoNetwork {
  pub switches: Vec<SwitchSpec>,
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

  pub fn build(self) -> IoNetwork {
    let mut switches = Vec::new();
    let mut driver_pins = Vec::new();
    let mut switch_offset = 0;
    let mut driver_offset = 0;

    for (i, spec) in self.specs.into_iter().enumerate() {
      for (idx, name) in spec.switch_map.iter() {
        let config = spec.switch_configs.get(name);

        switches.push(SwitchSpec {
          id: switch_offset as usize + *idx as usize,
          name: *name,
          parent_index: i as u8,
          config: config.cloned(),
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

    IoNetwork {
      switches,
      driver_pins,
    }
  }
}

#[derive(Debug, Clone)]
pub struct SwitchSpec {
  pub id: usize,
  pub name: &'static str,
  pub parent_index: u8,
  pub config: Option<SwitchConfig>,
}

#[derive(Debug, Clone)]
pub struct SwitchConfig {
  pub inverted: bool,
  pub debounce_close: Option<Duration>,
  pub debounce_open: Option<Duration>,
}

impl Default for SwitchConfig {
  fn default() -> Self {
    Self {
      inverted: false,
      debounce_close: None,
      debounce_open: None,
    }
  }
}

#[derive(Debug, Clone)]
pub struct DriverPin {
  pub id: usize,
  pub name: &'static str,
  pub parent_index: u8,
}
