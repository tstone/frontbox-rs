use core::panic;
use std::collections::HashMap;

use bevy_ecs::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct IoBoardSpec {
  pub switch_count: u32,
  pub driver_count: u32,
  pub switch_map: HashMap<u16, String>,
  pub driver_map: HashMap<u16, String>,
}

impl IoBoardSpec {
  pub fn with_switch(mut self, idx: u16, name: &str) -> Self {
    if idx >= self.switch_count as u16 {
      panic!(
        "Switch index {} out of bounds for board with {} switches",
        idx, self.switch_count
      );
    }

    self.switch_map.insert(idx, name.to_string());
    self
  }

  pub fn with_driver(mut self, idx: u16, name: &str) -> Self {
    if idx >= self.driver_count as u16 {
      panic!(
        "Driver index {} out of bounds for board with {} drivers",
        idx, self.driver_count
      );
    }

    self.driver_map.insert(idx, name.to_string());
    self
  }
}

pub struct FastIoBoards;

impl FastIoBoards {
  pub fn custom(switch_count: u32, driver_count: u32) -> IoBoardSpec {
    IoBoardSpec {
      switch_count,
      driver_count,
      ..Default::default()
    }
  }

  pub fn io_3208() -> IoBoardSpec {
    IoBoardSpec {
      switch_count: 32,
      driver_count: 8,
      ..Default::default()
    }
  }

  pub fn io_1616() -> IoBoardSpec {
    IoBoardSpec {
      switch_count: 16,
      driver_count: 16,
      ..Default::default()
    }
  }

  pub fn io_0804() -> IoBoardSpec {
    IoBoardSpec {
      switch_count: 8,
      driver_count: 4,
      ..Default::default()
    }
  }

  pub fn cabinet() -> IoBoardSpec {
    IoBoardSpec {
      switch_count: 24,
      driver_count: 8,
      ..Default::default()
    }
  }
}

#[derive(Component, Debug, Clone)]
pub struct IoBoard {
  pub index: u8,
  pub switch_offset: u32,
  pub driver_offset: u32,
  pub switch_count: u32,
  pub driver_count: u32,
  pub switch_map: HashMap<u16, String>,
  pub driver_map: HashMap<u16, String>,
}

#[derive(Debug, Clone)]
pub struct IoNetwork {
  pub boards: Vec<(String, IoBoard)>,
}

pub struct IoNetworkSpec {
  specs: Vec<(String, IoBoardSpec)>,
}

impl IoNetworkSpec {
  pub fn new() -> Self {
    Self { specs: Vec::new() }
  }

  pub fn add_board(&mut self, name: &str, spec: IoBoardSpec) {
    self.specs.push((name.to_string(), spec));
  }

  pub fn build(self) -> IoNetwork {
    let mut boards = Vec::new();
    let mut switch_offset = 0;
    let mut driver_offset = 0;

    for (i, (name, spec)) in self.specs.into_iter().enumerate() {
      boards.push((
        name.clone(),
        IoBoard {
          index: i as u8,
          switch_count: spec.switch_count,
          driver_count: spec.driver_count,
          switch_offset,
          driver_offset,
          switch_map: spec.switch_map,
          driver_map: spec.driver_map,
        },
      ));

      switch_offset += spec.switch_count;
      driver_offset += spec.driver_count;
    }

    IoNetwork { boards }
  }
}

#[macro_export]
macro_rules! define_io_network {
    ( $( $board_name:ident : $board_type:expr => {
        $( switches { $( $s_idx:literal : $s_name:ident ),* $(,)? } )?
        $( drivers { $( $d_idx:literal : $d_name:ident ),* $(,)? } )?
    } ),* ) => {
        {
            let mut network_spec = IoNetworkSpec::new();
            $(
                #[allow(unused_mut)]
                // $board_type is a Token Tree, so IoBoardSpec::cabinet() works perfectly here
                let mut spec = $board_type;
                $( $(
                    spec = spec.with_switch($s_idx, stringify!($s_name));
                )* )?
                $( $(
                    spec = spec.with_driver($d_idx, stringify!($d_name));
                )* )?
                network_spec.add_board(stringify!($board_name), spec);
            )*
            network_spec.build()
        }
    };
}
