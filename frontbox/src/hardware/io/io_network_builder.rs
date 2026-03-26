use crate::hardware::io::*;

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
    let mut boards: Vec<IoBoard> = Vec::new();
    let mut switches = Vec::new();
    let mut drivers = Vec::new();
    let mut switch_offset = 0;
    let mut driver_offset = 0;

    for (i, board) in self.boards.iter().enumerate() {
      boards.push(IoBoard {
        description: board.description,
        switch_count: board.switch_count,
        driver_count: board.driver_count,
      });

      for def in &board.switches {
        let mut def = def.clone();
        // re-write IDs to be sequential along I/O network
        def.id = switch_offset as usize + def.id;
        def.native.board_idx = i;
        switches.push(def);
      }

      switch_offset += board.switch_count;
    }

    for (i, board) in self.boards.into_iter().enumerate() {
      for def in board.drivers {
        let mut def = def.clone();
        // re-write IDs to be sequential along I/O network        def.id = driver_offset as usize + def.id;
        def.native.board_idx = i;
        def.id = driver_offset as usize + def.id;
        drivers.push(def);
      }
      driver_offset += board.driver_count;
    }

    IoNetwork {
      boards,
      switches,
      drivers,
    }
  }
}
