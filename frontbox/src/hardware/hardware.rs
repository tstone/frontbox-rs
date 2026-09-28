use crate::prelude::*;

#[derive(Clone, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Hardware {
  pub switches: SwitchLookup,
  pub drivers: DriverLookup,
  pub leds: LedLookup,
  pub io_network: Vec<ResolvedIoBoard>,
  pub exp_network: Vec<ResolvedExpansionBoard>,
}

impl Hardware {
  pub fn new(
    switches: SwitchLookup,
    drivers: DriverLookup,
    leds: LedLookup,
    io_network: Vec<ResolvedIoBoard>,
    exp_network: Vec<ResolvedExpansionBoard>,
  ) -> Self {
    Self {
      switches,
      drivers,
      leds,
      io_network,
      exp_network,
    }
  }

  /// Take the user-defined expansion board configurations and resolve actual hardware indexes/addresses
  pub fn resolve_expansion_boards(expansion_boards: &Vec<ExpBoard>) -> Vec<ResolvedExpansionBoard> {
    let mut resolved_boards = Vec::new();
    for board in expansion_boards {
      if board.model == FastExpansionBoardModels::Neuron {
        log::warn!(
          "Remapping Neuron LED ports to default 32-LED configuration for backwards compatibility"
        );
      }

      // sum up actual LEDs present and calculate index offsets
      let mut offset = 0;
      let mut resolved_ports = Vec::new();
      for port_idx in 0..board.hardware_led_port_count.unwrap_or(0) {
        // For EXP boards which have multiple LED port banks, the offset needs to be reset every 5th port
        if port_idx > 0 && port_idx % 4 == 0 {
          offset = 0;
        }

        if let Some(port) = board.led_ports.get(&port_idx) {
          let port_count_override = match board.model {
            // Known bug: Older versions of the Neuron have an internal Exp board which does not respect ER:
            FastExpansionBoardModels::Neuron => Some(32),
            _ => None,
          };

          let port =
            Self::resolve_led_port(board, port, port_idx as u8, offset, port_count_override);
          offset += port.length as u16;
          resolved_ports.push(port);
        } else {
          // no port defined = assume the default (32 LEDs)
          resolved_ports.push(ResolvedLedPort::default(offset));
          offset += 32;
        }
      }

      resolved_boards.push(ResolvedExpansionBoard {
        address: board.address,
        breakout: board.breakout,
        led_ports: resolved_ports,
        model: board.model,
      });
    }
    resolved_boards
  }

  fn resolve_led_port(
    board: &ExpBoard,
    port: &LedPort,
    port_idx: u8,
    offset: u16,
    port_count_override: Option<u8>,
  ) -> ResolvedLedPort {
    let mut port_led_total_count: u8 = 0;
    let mut addressed_leds = Vec::new();
    for multi_def in &port.leds {
      multi_def
        .children()
        .iter()
        .enumerate()
        .for_each(|(i, led)| {
          addressed_leds.push(ExpAddressed {
            definition: led.clone(),
            assignment: ExpAddress {
              board_address: board.address,
              breakout: Some(port_idx / 4),
              port: port_idx as u8,
            },
            id: i + port_led_total_count as usize + offset as usize,
          });
        });

      port_led_total_count = port_led_total_count.saturating_add(multi_def.children().len() as u8);
    }

    log::trace!("Mapped {} leds: {:?}", addressed_leds.len(), addressed_leds);

    ResolvedLedPort {
      led_type: port.led_type.clone(),
      offset,
      length: port_count_override.unwrap_or(port_led_total_count),
      leds: addressed_leds,
    }
  }
}
