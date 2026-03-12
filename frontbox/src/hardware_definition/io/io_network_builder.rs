use crate::hardware_definition::io::*;

// TODO: rename builder
pub struct IoNetworkBuilder {
  specs: Vec<IoBoardBuilder>,
}

impl IoNetworkBuilder {
  pub fn new() -> Self {
    Self { specs: Vec::new() }
  }

  pub fn add_board(&mut self, spec: IoBoardBuilder) {
    self.specs.push(spec);
  }

  pub fn build(self) -> IoNetwork {
    let mut boards: Vec<IoBoardDefinition> = Vec::new();
    let mut switches = Vec::new();
    let mut driver_pins = Vec::new();
    let mut switch_offset = 0;
    let mut driver_offset = 0;

    for (i, spec) in self.specs.into_iter().enumerate() {
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
      }

      for (idx, name) in spec.driver_map.iter() {
        driver_pins.push(DriverDefinition {
          id: driver_offset as usize + *idx as usize,
          name: *name,
          parent_index: i as u8,
          config: spec.driver_configs.get(name).cloned(),
        });
      }

      switch_offset += spec.switch_count;
      driver_offset += spec.driver_count;
    }

    IoNetwork {
      boards,
      switches,
      drivers: driver_pins,
    }
  }
}
