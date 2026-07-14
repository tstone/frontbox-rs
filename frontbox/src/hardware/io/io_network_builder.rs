use crate::hardware::io::*;
use crate::prelude::*;

pub struct IoNetworkBuilder {
  pub(crate) boards: Vec<IoBoardBuilder>,
}

impl IoNetworkBuilder {
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

      for wired in &board.switches {
        let pin = wired.assignment.pin;
        let addressed = IoAddressed {
          definition: wired.definition.clone(),
          assignment: IoAddress::new(i as u8, pin),
          // write IDs to be sequential along I/O network
          id: switch_offset as usize + pin as usize,
        };
        switches.push(addressed);
      }

      switch_offset += board.switch_count;
    }

    for (i, board) in self.boards.into_iter().enumerate() {
      for wired in board.drivers {
        let pin = wired.assignment.pin;
        let addressed = IoAddressed {
          definition: wired.definition.clone(),
          assignment: IoAddress::new(i as u8, pin),
          // write IDs to be sequential along I/O network
          id: driver_offset as usize + pin as usize,
        };
        drivers.push(addressed);
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
