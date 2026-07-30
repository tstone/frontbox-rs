use std::time::Duration;

use fast_protocol::*;

use crate::machine::serial_interface::SerialInterface;
use crate::prelude::*;

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

  /// Read the hardware state of all switches at startup to initialize the switch context
  pub async fn get_initial_switch_states(io_port: &mut SerialInterface) -> Vec<SwitchState> {
    match io_port
      .request(&ReportSwitchesCommand, Duration::from_millis(2000))
      .await
      .unwrap()
    {
      SwitchReportResponse::SwitchReport { switches } => switches,
      other => panic!(
        "Unexpected response while requesting initial switch states: {:?}",
        other
      ),
    }
  }

  pub async fn configure_drivers(io_port: &mut SerialInterface, ctx: &ContextBase) {
    for driver in ctx.drivers.values() {
      if let Some(mode) = ctx.drivers.config(driver.name) {
        log::info!("Configuring driver {} with {:?}", driver.name, mode);
        match io_port
          .request(
            &ConfigureDriverCommand::new(driver.id, mode.to_config(ctx)),
            Duration::from_millis(500),
          )
          .await
        {
          Ok(ProcessedResponse::Processed) => {
            log::debug!("Driver {} configured successfully", driver.name);
          }
          Ok(ProcessedResponse::Failed) => {
            panic!("Driver {} configuration failed", driver.name);
          }
          Err(e) => {
            panic!("Error configuring driver {}: {}", driver.name, e);
          }
        }
      }
    }
  }

  pub async fn reset_expansion_boards(
    exp_port: &mut SerialInterface,
    expansion_boards: &Vec<ResolvedExpansionBoard>,
  ) {
    for board in expansion_boards {
      Self::reset_expansion_board(exp_port, board).await;
    }
  }

  pub async fn reset_expansion_board(
    exp_port: &mut SerialInterface,
    board: &ResolvedExpansionBoard,
  ) {
    if board.breakout.is_none() {
      log::info!("Resetting expansion board at address {:X}", board.address);
      match exp_port
        .request(
          &BoardResetCommand::new(board.address),
          Duration::from_millis(2000),
        )
        .await
      {
        Ok(ProcessedResponse::Processed) => {
          log::debug!("Expansion board {:X} reset successfully", board.address);
        }
        Ok(ProcessedResponse::Failed) => {
          panic!(
            "Expansion board {:X} reset failed. Is this configured correctly?",
            board.address
          );
        }
        Err(e) => {
          panic!("Error resetting expansion board {:X}: {}", board.address, e);
        }
      }
    }
  }

  /// Query the I/O network to resolve actual hardware configurations (switch/driver counts, versions, etc) for each board
  /// Verify that the actual hardware matches the user-defined configuration. This also loads firmware and PCB version for
  /// cases where minimum supported versions need to be checked.
  pub async fn resolve_io_network(
    io_port: &mut SerialInterface,
    io_network: &IoNetwork,
  ) -> ResolvedIoNetwork {
    let mut resolved_boards = Vec::new();

    for (id, board) in io_network.boards.iter().enumerate() {
      // query each board for its actual hardware configuration (switch/driver counts, version, etc)
      let response = io_port
        .request(&NodeNameCommand::new(id as u8), Duration::from_millis(500))
        .await;
      match response {
        Ok(NodeInfo::Success {
          node_id,
          name,
          board_revision,
          firmware_version,
          driver_count,
          switch_count,
        }) => {
          assert!(
            driver_count == board.driver_count,
            "Driver count mismatch for board {}: expected {}, got {}. Boards may be misconfigured or inserted out of order",
            board.description,
            board.driver_count,
            driver_count,
          );
          assert!(
            switch_count == board.switch_count,
            "Switch count mismatch for board {}: expected {}, got {}. Boards may be misconfigured or inserted out of order",
            board.description,
            board.switch_count,
            switch_count,
          );

          log::info!(
            "Confirmed I/O node board {} {} v{} ({})",
            node_id,
            name,
            firmware_version,
            board.description
          );

          resolved_boards.push(ResolvedIoBoard {
            node_id,
            description: board.description,
            name,
            firmware_version,
            board_revision,
            switch_count,
            driver_count,
          });
        }
        other => panic!(
          "Unexpected response while querying board info for {}: {:?}",
          board.description, other
        ),
      }
    }

    ResolvedIoNetwork {
      boards: resolved_boards,
    }
  }

  /// Take the user-defined expansion board configurations and resolve actual hardware indexes/addresses
  pub fn resolve_expansion_boards(
    expansion_boards: &Vec<ExpBoard>,
  ) -> Vec<ResolvedExpansionBoard> {
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

  pub async fn configure_led_ports(
    exp_port: &mut SerialInterface,
    expansion_boards: &Vec<ResolvedExpansionBoard>,
  ) {
    for board in expansion_boards {
      for (port_index, led_port) in board.led_ports.iter().enumerate() {
        Self::configure_led_port(exp_port, board, port_index as u8, led_port.offset, led_port)
          .await;
      }
    }
  }

  pub async fn configure_led_port(
    exp_port: &mut SerialInterface,
    board: &ResolvedExpansionBoard,
    port_index: u8,
    led_index_offset: u16,
    led_port: &ResolvedLedPort,
  ) {
    let cmd = ConfigureLedPortCommand::new(
      board.address,
      board.breakout,
      port_index,
      led_port.led_type.clone(),
      led_index_offset,
      led_port.length,
    );
    // configure port/block
    let _ = exp_port.request(&cmd, Duration::from_millis(250)).await;
  }
}
