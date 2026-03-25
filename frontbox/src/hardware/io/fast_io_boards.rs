use std::collections::HashMap;

use crate::IoBoardBuilder;

/// Pre-built definitions of FAST IO board configurations
pub struct FastIoBoards;

impl FastIoBoards {
  pub fn custom(switch_count: u32, driver_count: u32) -> IoBoardBuilder {
    IoBoardBuilder {
      description: Box::leak(
        format!(
          "Custom IO Board ({} switches, {} drivers)",
          switch_count, driver_count
        )
        .into_boxed_str(),
      ),
      switch_count,
      driver_count,
      switch_map: HashMap::new(),
      driver_map: HashMap::new(),
      switch_configs: HashMap::new(),
      driver_configs: HashMap::new(),
    }
  }

  pub fn io_3208() -> IoBoardBuilder {
    IoBoardBuilder {
      description: "IO-3208",
      switch_count: 32,
      driver_count: 8,
      switch_map: HashMap::new(),
      driver_map: HashMap::new(),
      switch_configs: HashMap::new(),
      driver_configs: HashMap::new(),
    }
  }

  pub fn io_1616() -> IoBoardBuilder {
    IoBoardBuilder {
      description: "IO-1616",
      switch_count: 16,
      driver_count: 16,
      switch_map: HashMap::new(),
      driver_map: HashMap::new(),
      switch_configs: HashMap::new(),
      driver_configs: HashMap::new(),
    }
  }

  pub fn io_0804() -> IoBoardBuilder {
    IoBoardBuilder {
      description: "IO-0804",
      switch_count: 8,
      driver_count: 4,
      switch_map: HashMap::new(),
      driver_map: HashMap::new(),
      switch_configs: HashMap::new(),
      driver_configs: HashMap::new(),
    }
  }

  pub fn cabinet() -> IoBoardBuilder {
    IoBoardBuilder {
      description: "Cabinet IO",
      switch_count: 24,
      driver_count: 8,
      switch_map: HashMap::new(),
      driver_map: HashMap::new(),
      switch_configs: HashMap::new(),
      driver_configs: HashMap::new(),
    }
  }
}
