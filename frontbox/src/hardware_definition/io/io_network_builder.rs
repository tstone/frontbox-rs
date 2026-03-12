use std::collections::HashMap;

use crate::hardware_definition::io::*;

// TODO: rename builder
pub struct IoNetworkBuilder {
  boards: Vec<IoBoardBuilder>,
}

impl IoNetworkBuilder {
  pub fn new() -> Self {
    Self { boards: Vec::new() }
  }

  pub fn add_board(&mut self, spec: IoBoardBuilder) {
    self.boards.push(spec);
  }

  pub fn build(self) -> IoNetwork {
    let mut boards: Vec<IoBoardDefinition> = Vec::new();
    let mut switches = Vec::new();
    let mut driver_pins = Vec::new();
    let mut switch_lookup: HashMap<&'static str, usize> = HashMap::new();
    let mut switch_offset = 0;
    let mut driver_offset = 0;

    // first process all switches, as those will need to be referenced by drivers
    for (i, spec) in self.boards.iter().enumerate() {
      boards.push(IoBoardDefinition {
        description: spec.description,
        switch_count: spec.switch_count,
        driver_count: spec.driver_count,
      });

      for (idx, name) in spec.switch_map.iter() {
        let config = spec.switch_configs.get(name);

        switches.push(SwitchDefinition {
          id: switch_offset as usize + *idx as usize,
          name: *name,
          parent_index: i as u8,
          config: config.cloned(),
        });

        switch_lookup.insert(*name, switch_offset as usize + *idx as usize);
      }

      switch_offset += spec.switch_count;
    }

    for (i, spec) in self.boards.into_iter().enumerate() {
      for (idx, name) in spec.driver_map.iter() {
        driver_pins.push(DriverDefinition {
          id: driver_offset as usize + *idx as usize,
          name: *name,
          parent_index: i as u8,
          config: spec
            .driver_configs
            .get(name)
            .map(|c| c.to_config(&switch_lookup)),
        });
      }
      driver_offset += spec.driver_count;
    }

    IoNetwork {
      boards,
      switches,
      drivers: driver_pins,
    }
  }
}
