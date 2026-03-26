use std::time::Duration;

use fast_protocol::*;

use crate::machine::serial_interface::SerialInterface;
use crate::prelude::*;

pub struct Hardware {
  pub switches: SwitchLookup,
  pub drivers: DriverLookup,
  pub io_network: Vec<IoBoard>,
  pub exp_network: Vec<ExpansionBoard>,
}

impl Hardware {
  pub fn new(
    switches: SwitchLookup,
    drivers: DriverLookup,
    io_network: Vec<IoBoard>,
    exp_network: Vec<ExpansionBoard>,
  ) -> Self {
    Self {
      switches,
      drivers,
      io_network,
      exp_network,
    }
  }

  /// Read the hardware state of all switches at startup to initialize the switch context
  pub async fn get_initial_switch_states(io_port: &mut SerialInterface) -> Vec<SwitchState> {
    io_port
      .request_until_match(
        ReportSwitchesCommand::new(),
        Duration::from_millis(2000),
        |resp| {
          if let SwitchReportResponse::SwitchReport { switches } = resp {
            log::debug!("🥾 Initial switch states: {:?}", switches);
            Some(switches)
          } else {
            None
          }
        },
      )
      .await
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
            &ConfigureDriverCommand::new(&driver.id, &mode.to_config(switch_lookup)),
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
    expansion_boards: &Vec<ExpansionBoard>,
  ) {
    for board in expansion_boards {
      Self::reset_expansion_board(exp_port, board).await;
    }
  }

  pub async fn reset_expansion_board(exp_port: &mut SerialInterface, board: &ExpansionBoard) {
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

  pub async fn configure_led_ports(
    exp_port: &mut SerialInterface,
    expansion_boards: &Vec<ExpansionBoard>,
  ) {
    for board in expansion_boards {
      for led_port in &board.led_ports {
        Self::configure_led_port(exp_port, board, led_port).await;
      }
    }
  }

  pub async fn configure_led_port(
    exp_port: &mut SerialInterface,
    board: &ExpansionBoard,
    led_port: &LedPort,
  ) {
    let cmd = ConfigureLedPortCommand::new(
      board.address,
      board.breakout,
      led_port.port,
      led_port.led_type.clone(),
      led_port.start,
      led_port.leds.len() as u8,
    );
    // configure port/block
    let _ = exp_port.request(&cmd, Duration::from_millis(250)).await;
  }
}
