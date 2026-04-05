use std::time::Duration;

use fast_protocol::*;
use itertools::Itertools;

use crate::machine::serial_interface::SerialInterface;
use crate::prelude::*;

pub struct Hardware {
  pub switches: SwitchLookup,
  pub drivers: DriverLookup,
  pub illuminations: IlluminationLookup,
  pub io_network: Vec<IoBoard>,
  pub exp_network: Vec<ResolvedExpansionBoard>,
}

impl Hardware {
  pub fn new(
    switches: SwitchLookup,
    drivers: DriverLookup,
    illuminations: IlluminationLookup,
    io_network: Vec<IoBoard>,
    exp_network: Vec<ResolvedExpansionBoard>,
  ) -> Self {
    Self {
      switches,
      drivers,
      illuminations,
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

  pub async fn configure_drivers(
    io_port: &mut SerialInterface,
    drivers: &Vec<DriverDefinition>,
    switch_lookup: &SwitchLookup,
  ) {
    for driver in drivers {
      if let Some(mode) = &driver.mode {
        log::info!("Configuring driver {} with {:?}", driver.name, mode);
        match io_port
          .request(
            &ConfigureDriverCommand::new(driver.id, mode.to_config(switch_lookup)),
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

  /// Take the user-defined expansion board configurations and resolve actual hardware indexes/addresses
  pub fn resolve_expansion_boards(
    expansion_boards: &Vec<ExpansionBoard>,
  ) -> Vec<ResolvedExpansionBoard> {
    let mut resolved_boards = Vec::new();
    for board in expansion_boards {
      let mut resolved_ports = Vec::new();
      let mut offset = 0;

      // sum up actual LEDs present and calculate index offsets
      for idx in 0..board.hardware_led_port_count.unwrap_or(0) {
        if let Some(port) = board.led_ports.get(&idx) {
          let mut port_led_total_count: u8 = 0;
          let mut resolved_illuminations = Vec::new();
          for illum in &port.illuminations {
            port_led_total_count += illum.led_count();

            let addressable_leds = (offset..offset + illum.led_count() as u16)
              .map(|i| AddressableLed {
                address: LedAddress {
                  address: board.address,
                  breakout: board.breakout,
                  port: idx as u8,
                },
                index: i,
              })
              .collect_vec();

            resolved_illuminations.push(AddressableIllumination {
              leds: addressable_leds,
              source: illum.clone(),
            });
          }
          resolved_ports.push(ResolvedLedPort {
            led_type: port.led_type.clone(),
            start: offset,
            length: port_led_total_count,
            illuminations: resolved_illuminations,
          });

          offset += port_led_total_count as u16;
        } else {
          // no port defined = assume the default (32 LEDs)
          resolved_ports.push(ResolvedLedPort {
            led_type: LedType::WS2812,
            start: offset,
            length: 32,
            illuminations: Vec::new(),
          });
          offset += 32;
        }
      }

      resolved_boards.push(ResolvedExpansionBoard {
        address: board.address,
        breakout: board.breakout,
        led_ports: resolved_ports,
      });
    }
    resolved_boards
  }

  pub async fn configure_led_ports(
    exp_port: &mut SerialInterface,
    expansion_boards: &Vec<ResolvedExpansionBoard>,
  ) {
    for board in expansion_boards {
      for (port_index, led_port) in board.led_ports.iter().enumerate() {
        Self::configure_led_port(exp_port, board, port_index as u8, led_port).await;
      }
    }
  }

  pub async fn configure_led_port(
    exp_port: &mut SerialInterface,
    board: &ResolvedExpansionBoard,
    port_index: u8,
    led_port: &ResolvedLedPort,
  ) {
    let cmd = ConfigureLedPortCommand::new(
      board.address,
      board.breakout,
      port_index,
      led_port.led_type.clone(),
      led_port.length,
      led_port.length,
    );
    // configure port/block
    let _ = exp_port.request(&cmd, Duration::from_millis(250)).await;
  }
}
