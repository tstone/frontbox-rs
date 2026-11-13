use std::time::Duration;

use fast_protocol::exp::prelude::*;
use fast_protocol::net::prelude::*;
use fast_protocol::{ProcessedResponse, SwitchState};

use crate::machine::neuron::BootError;
use crate::machine::serial_interface::{SerialError, SerialInterface};
use crate::prelude::*;

const RETRY_DELAY: Duration = Duration::from_millis(500);
pub(crate) const MAX_RETRY_ATTEMPTS: u32 = 5;
pub const MAINBOARD_HANDSHAKE_ATTEMPTS: u32 = 60;

/// Run a boot step until it succeeds, up to `MAX_RETRY_ATTEMPTS` times or the number given first
macro_rules! retry {
  ($step:expr) => {
    $crate::machine::neuron::boot::retry!($crate::machine::neuron::boot::MAX_RETRY_ATTEMPTS, $step)
  };
  ($max_attempts:expr, $step:expr) => {{
    let mut attempt = 1;
    loop {
      match $step.await {
        Ok(value) => break Ok(value),
        Err(error) => {
          match $crate::machine::neuron::boot::after_failed_attempt(error, attempt, $max_attempts)
            .await
          {
            Ok(()) => attempt += 1,
            Err(error) => break Err(error),
          }
        }
      }
    }
  }};
}
pub(crate) use retry;

/// After a boot step fails: wait to try it again, or give back the error if it's out of attempts or trying again won't
/// help
pub(crate) async fn after_failed_attempt(
  error: BootError,
  attempt: u32,
  max_attempts: u32,
) -> Result<(), BootError> {
  if !error.is_retryable() || attempt >= max_attempts {
    return Err(error);
  }
  log::warn!(target: "frontbox::machine::boot", "🥾 {error} (attempt {attempt} of {max_attempts}), retrying...");
  tokio::time::sleep(RETRY_DELAY).await;
  Ok(())
}

/// For the first boot, where there's no running game to fall back to: stop with the error
pub fn boot_failed<T>(error: BootError) -> T {
  panic!("🥾 {error}")
}

/// What's needed to bring up the hardware, kept to bring it up again after the connection is lost
pub struct NeuronSetup {
  pub io_net_port_path: &'static str,
  pub exp_port_path: &'static str,
  pub io_network: IoNetwork,
  pub expansion_boards: Vec<ResolvedExpansionBoard>,
}

/// The hardware brought up and ready to run, and what was read from it on the way
pub struct NeuronConnection {
  pub io_port: SerialInterface,
  pub exp_port: SerialInterface,
  pub io_network: ResolvedIoNetwork,
  pub switch_states: Vec<SwitchState>,
}

pub async fn connect(setup: &NeuronSetup) -> Result<NeuronConnection, BootError> {
  let mut io_port = open(setup.io_net_port_path).await?;
  retry!(MAINBOARD_HANDSHAKE_ATTEMPTS, handshake(&mut io_port))?;

  // Verify user-configuration and load firmware/board versions
  let io_network = retry!(resolve_io_network(&mut io_port, &setup.io_network))?;
  verify_watchdog(&mut io_port).await?;
  configure_switches(&mut io_port, &setup.io_network.switches).await;
  let switch_states = retry!(read_switch_states(&mut io_port))?;

  let mut exp_port = open(setup.exp_port_path).await?;
  retry!(reset_expansion_boards(
    &mut exp_port,
    &setup.expansion_boards
  ))?;
  configure_led_ports(&mut exp_port, &setup.expansion_boards).await;

  // the steps that only warn carry on past a lost port, so check nothing was lost along the way
  if !io_port.is_connected() || !exp_port.is_connected() {
    return Err(BootError::PortLost {
      step: "Bringing up the hardware".into(),
    });
  }

  Ok(NeuronConnection {
    io_port,
    exp_port,
    io_network,
    switch_states,
  })
}

async fn open(port_path: &'static str) -> Result<SerialInterface, BootError> {
  let port = SerialInterface::new(port_path)
    .await
    .map_err(|e| BootError::OpenFailed {
      port: port_path.into(),
      reason: e.to_string(),
    })?;
  log::info!(target: "frontbox::machine::boot", "🥾 Opened port {}", port_path);
  Ok(port)
}

/// Check the mainboard is ready to respond (ID), then set it up for this platform (CH). Run it with
/// `retry!(MAINBOARD_HANDSHAKE_ATTEMPTS, ..)` to wait for a mainboard that's still powering up.
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
    log::warn!(target: "frontbox::machine::boot", "{e}");
  }

  log::info!(target: "frontbox::machine::boot", "🥾 Mainboard is ready");
  Ok(mainboard_name)
}

/// Verify the watchdog is responsive. Sometimes the first few commands will fail. Only gives up if the port is lost.
pub async fn verify_watchdog(io_port: &mut SerialInterface) -> Result<(), BootError> {
  loop {
    match io_port
      .query(
        &WatchdogCommand::set(Duration::from_millis(1250)),
        Duration::from_secs(1),
      )
      .await
    {
      Ok(WatchdogResponse::Processed) => {
        log::info!(target: "frontbox::machine::boot", "🥾 Watchdog is ready");
        return Ok(());
      }
      Err(SerialError::Lost) => {
        return Err(BootError::PortLost {
          step: "Verifying the watchdog".into(),
        });
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
        target: "frontbox::machine::boot",
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
        log::warn!(target: "frontbox::machine::boot", "{e}");
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
      log::info!(target: "frontbox::machine::boot", "Configuring driver {} with {:?}", driver.name, mode);
      let result = io_port
        .command(
          &ConfigureDriverCommand::new(driver.id, mode.to_config(state)),
          Duration::from_millis(500),
        )
        .await;
      BootError::processed(|| format!("Configuring driver {}", driver.name), result)?;
      log::debug!(target: "frontbox::machine::boot", "Driver {} configured successfully", driver.name);
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
    log::info!(target: "frontbox::machine::boot", "Resetting expansion board at address {:X}", board.address);
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
    log::debug!(target: "frontbox::machine::boot", "Expansion board {:X} reset successfully", board.address);
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
          target: "frontbox::machine::boot",
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
        target: "frontbox::machine::boot",
        "LED port {} on board {:X} configured with {} LEDs from {}",
        port_index + 1,
        board.address,
        led_port.length,
        led_index_offset
      );
    }
    Ok(ProcessedResponse::Failed) => {
      log::warn!(
        target: "frontbox::machine::boot",
        "LED port {} on board {:X} configuration ({} LEDs from {}) failed; its LEDs may be misaddressed",
        port_index + 1,
        board.address,
        led_port.length,
        led_index_offset
      );
    }
    Err(e) => {
      log::warn!(
        target: "frontbox::machine::boot",
        "Error configuring LED port {} on board {:X}: {}; its LEDs may be misaddressed",
        port_index + 1,
        board.address,
        e
      );
    }
  }
}
