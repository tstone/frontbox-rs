use std::time::Duration;

use fast_protocol::exp::prelude::*;
use fast_protocol::net::prelude::*;
use fast_protocol::{ProcessedResponse, SwitchState};

use crate::machine::neuron::BootError;
use crate::machine::serial_interface::SerialInterface;
use crate::prelude::*;

const RETRY_DELAY: Duration = Duration::from_millis(500);
const MAX_RETRY_ATTEMPTS: u32 = 5;
pub const MAINBOARD_HANDSHAKE_ATTEMPTS: u32 = 60;

/// Run a boot step until it succeeds, up to `MAX_ATTEMPTS` times
pub async fn retry<T>(step: impl AsyncFnMut() -> Result<T, BootError>) -> Result<T, BootError> {
  retry_up_to(MAX_RETRY_ATTEMPTS, step).await
}

/// Run a boot step until it succeeds, up to `max_attempts` times. Gives up with the last error once the attempts run
/// out, or straight away if trying again won't help (the hardware doesn't match the configuration, or the port is
/// gone).
pub async fn retry_up_to<T>(
  max_attempts: u32,
  mut step: impl AsyncFnMut() -> Result<T, BootError>,
) -> Result<T, BootError> {
  let mut attempt = 1;
  loop {
    match step().await {
      Ok(value) => return Ok(value),
      Err(e) if !e.is_retryable() || attempt >= max_attempts => return Err(e),
      Err(e) => {
        log::warn!("🥾 {e} (attempt {attempt} of {max_attempts}), retrying...");
        attempt += 1;
        tokio::time::sleep(RETRY_DELAY).await;
      }
    }
  }
}

/// For the first boot, where there's no running game to fall back to: stop with the error
pub fn boot_failed<T>(error: BootError) -> T {
  panic!("🥾 {error}")
}

/// Check the mainboard is ready to respond (ID), then set it up for this platform (CH). Run it with
/// `retry_up_to(HANDSHAKE_ATTEMPTS, ..)` to wait for a mainboard that's still powering up.
pub async fn handshake(io_port: &mut SerialInterface) -> Result<String, BootError> {
  const STEP: &str = "Reaching the mainboard";
  let mainboard_name = match io_port
    .query(&IdCommand::new(), Duration::from_millis(500))
    .await
  {
    Ok(IdResponse::Report { mainboard_name, .. }) => mainboard_name,
    Ok(IdResponse::Failed) => return Err(BootError::Failed { step: STEP.into() }),
    Err(error) => {
      return Err(BootError::no_response(STEP.into(), error));
    }
  };

  // Initialize the mainboard with the right firmware
  let result = io_port
    .command(
      &ConfigureHardwareCommand::new(FastPlatform::Neuron.into(), Some(SwitchReporting::Verbose)),
      Duration::from_millis(500),
    )
    .await;
  if let Err(e) = BootError::processed(|| "Configuring the mainboard".into(), result) {
    log::warn!("{e}");
  }

  log::info!("🥾 Mainboard is ready");
  Ok(mainboard_name)
}

/// Verify the watchdog is responsive. Sometimes the first few commands will fail.
pub async fn verify_watchdog(io_port: &mut SerialInterface) {
  loop {
    match io_port
      .query(
        &WatchdogCommand::set(Duration::from_millis(1250)),
        Duration::from_secs(1),
      )
      .await
    {
      Ok(WatchdogResponse::Processed) => {
        log::info!("🥾 Watchdog is ready");
        break;
      }
      _ => {
        tokio::time::sleep(Duration::from_millis(500)).await;
      }
    };
  }
}

pub async fn configure_switches(
  io_port: &mut SerialInterface,
  switches: &Vec<IoAddressed<SwitchDefinition>>,
) {
  for switch in switches {
    if let Some(config) = &switch.definition.config {
      let reporting = if config.inverted {
        SwitchReportingMode::ReportInverted
      } else {
        SwitchReportingMode::ReportNormal
      };
      log::info!(
        "Configuring switch {} with {:?}",
        switch.definition.name,
        config
      );
      let result = io_port
        .command(
          &ConfigureSwitchCommand::new(
            switch.id,
            reporting,
            config.debounce_close,
            config.debounce_open,
          ),
          Duration::from_millis(500),
        )
        .await;
      if let Err(e) = BootError::processed(
        || format!("Configuring switch {}", switch.definition.name),
        result,
      ) {
        log::warn!("{e}");
      }
    }
  }
}

pub async fn configure_drivers(
  io_port: &mut SerialInterface,
  state: &AppState,
) -> Result<(), BootError> {
  for driver in state.drivers.values() {
    if let Some(mode) = state.drivers.mode(driver.name) {
      log::info!("Configuring driver {} with {:?}", driver.name, mode);
      let result = io_port
        .command(
          &ConfigureDriverCommand::new(driver.id, mode.to_config(state)),
          Duration::from_millis(500),
        )
        .await;
      BootError::processed(|| format!("Configuring driver {}", driver.name), result)?;
      log::debug!("Driver {} configured successfully", driver.name);
    }
  }
  Ok(())
}

/// Read the hardware state of all switches
pub async fn read_switch_states(
  io_port: &mut SerialInterface,
) -> Result<Vec<SwitchState>, BootError> {
  const STEP: &str = "Reading switch states";
  match io_port
    .query(&ReportSwitchesCommand, Duration::from_millis(2000))
    .await
  {
    Ok(SwitchReportResponse::SwitchReport { switches }) => Ok(switches),
    Ok(SwitchReportResponse::Failed) => Err(BootError::Failed { step: STEP.into() }),
    Err(error) => Err(BootError::no_response(STEP.into(), error)),
  }
}

pub async fn reset_expansion_boards(
  exp_port: &mut SerialInterface,
  expansion_boards: &Vec<ResolvedExpansionBoard>,
) -> Result<(), BootError> {
  for board in expansion_boards {
    reset_expansion_board(exp_port, board).await?;
  }
  Ok(())
}

pub async fn reset_expansion_board(
  exp_port: &mut SerialInterface,
  board: &ResolvedExpansionBoard,
) -> Result<(), BootError> {
  if board.breakout.is_none() {
    log::info!("Resetting expansion board at address {:X}", board.address);
    let result = exp_port
      .command(
        &BoardResetCommand::new(board.address),
        Duration::from_millis(2000),
      )
      .await;
    BootError::processed(
      || {
        format!(
          "Resetting expansion board {:X} (is it configured correctly?)",
          board.address
        )
      },
      result,
    )?;
    log::debug!("Expansion board {:X} reset successfully", board.address);
  }
  Ok(())
}

/// Query the I/O network to resolve actual hardware configurations (switch/driver counts, versions, etc) for each board
/// Verify that the actual hardware matches the user-defined configuration. This also loads firmware and PCB version for
/// cases where minimum supported versions need to be checked.
pub async fn resolve_io_network(
  io_port: &mut SerialInterface,
  io_network: &IoNetwork,
) -> Result<ResolvedIoNetwork, BootError> {
  let mut resolved_boards = Vec::new();

  for (id, board) in io_network.boards.iter().enumerate() {
    let step = || format!("Querying board info for {}", board.description);

    // query each board for its actual hardware configuration (switch/driver counts, version, etc)
    let response = io_port
      .query(&NodeNameCommand::new(id as u8), Duration::from_millis(500))
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
        if driver_count != board.driver_count {
          return Err(BootError::Misconfigured(format!(
            "Driver count mismatch for board {}: expected {}, got {}. Boards may be misconfigured or inserted out of order",
            board.description, board.driver_count, driver_count,
          )));
        }
        if switch_count != board.switch_count {
          return Err(BootError::Misconfigured(format!(
            "Switch count mismatch for board {}: expected {}, got {}. Boards may be misconfigured or inserted out of order",
            board.description, board.switch_count, switch_count,
          )));
        }

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
      Ok(NodeInfo::Failed) => return Err(BootError::Failed { step: step() }),
      Err(error) => {
        return Err(BootError::no_response(step(), error));
      }
    }
  }

  Ok(ResolvedIoNetwork {
    boards: resolved_boards,
  })
}

/// Configure the LED ports on every expansion board. A port that can't be configured only misaddresses its own LEDs,
/// so this warns rather than failing.
pub async fn configure_led_ports(
  exp_port: &mut SerialInterface,
  expansion_boards: &Vec<ResolvedExpansionBoard>,
) {
  for board in expansion_boards {
    for (port_index, led_port) in board.led_ports.iter().enumerate() {
      // Configure every port, even ones with no LEDs: an unconfigured port keeps its default range (32 LEDs at
      // 32 * port), which overlaps the ranges given to the ports after it and the board rejects those.
      configure_led_port(exp_port, board, port_index as u8, led_port.offset, led_port).await;
    }
  }
}

pub async fn configure_led_port(
  exp_port: &mut SerialInterface,
  board: &ResolvedExpansionBoard,
  port_index: u8,
  led_index_offset: u8,
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
  match exp_port.command(&cmd, Duration::from_millis(250)).await {
    Ok(ProcessedResponse::Processed) => {
      log::debug!(
        "LED port {} on board {:X} configured with {} LEDs from {}",
        port_index + 1,
        board.address,
        led_port.length,
        led_index_offset
      );
    }
    Ok(ProcessedResponse::Failed) => {
      log::warn!(
        "LED port {} on board {:X} configuration ({} LEDs from {}) failed; its LEDs may be misaddressed",
        port_index + 1,
        board.address,
        led_port.length,
        led_index_offset
      );
    }
    Err(e) => {
      log::warn!(
        "Error configuring LED port {} on board {:X}: {}; its LEDs may be misaddressed",
        port_index + 1,
        board.address,
        e
      );
    }
  }
}
