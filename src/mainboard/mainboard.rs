use std::time::Duration;

use crate::mainboard::serial_interface::SerialInterface;
use crate::protocol::configure_hardware::SwitchReporting;
use crate::protocol::id::{IdCommand, IdResponse};
use crate::protocol::prelude::{
  ConfigureHardwareCommand, ReportSwitchesCommand, SwitchReportResponse, WatchdogCommand,
  WatchdogResponse,
};
use crate::protocol::{EventResponse, SwitchState};
use tokio::sync::mpsc;
use tokio::time::sleep;

/// Handles serial
pub struct Mainboard {
  commands_incoming: mpsc::Receiver<MainboardCommand>,
  events_outgoing: mpsc::Sender<MainboardIncoming>,
  io_port: SerialInterface,
  exp_port: SerialInterface,
  enable_watchdog: bool,
}

#[derive(Debug, Clone)]
pub struct MainboardIncoming {
  pub event: EventResponse,
  pub channel: FastChannel,
}

#[derive(Debug, Clone)]
pub enum FastChannel {
  Io,
  Expansion,
}

impl Mainboard {
  pub async fn boot(
    config: BootConfig,
    commands_incoming: mpsc::Receiver<MainboardCommand>,
    events_outgoing: mpsc::Sender<MainboardIncoming>,
  ) -> BootResult {
    // open IO port
    let mut io_port = SerialInterface::new(config.io_net_port_path)
      .await
      .expect("Failed to open IO NET port");

    log::info!("🥾 Opened IO NET port at {}", config.io_net_port_path);

    // open EXP port
    let exp_port = SerialInterface::new(config.exp_port_path)
      .await
      .expect("Failed to open EXP port");
    log::info!("🥾 Opened EXP port at {}", config.exp_port_path);

    // wait for mainboard ID response (boot cycle complete)
    let _ = io_port
      .request_until_match(IdCommand::new(), Duration::from_millis(2000), |response| {
        if let IdResponse::Report {
          processor,
          product_number,
          firmware_version,
        } = response
        {
          log::info!(
            "🥾 Connected to mainboard {} {} with firmware: {}",
            processor,
            product_number,
            firmware_version
          );
          Some(true)
        } else {
          None
        }
      })
      .await;

    // configure hardware (instruct mainboard which 'platform' it is)
    log::info!(
      "🥾 Configuring mainboard hardware as platform {:?}",
      config.platform,
    );
    let _ = io_port
      .request(
        &ConfigureHardwareCommand::new(
          config.platform.clone() as u16,
          Some(SwitchReporting::Verbose),
        ),
        Duration::from_millis(2000),
      )
      .await;

    // get initial switch state
    let switches = io_port
      .request_until_match(
        ReportSwitchesCommand::new(),
        Duration::from_millis(2000),
        |resp| {
          if let SwitchReportResponse::SwitchReport { switches } = resp {
            log::info!("🥾 Initial switch states: {:?}", switches);
            Some(switches)
          } else {
            None
          }
        },
      )
      .await;

    // verify watchdog is ready
    let _ = io_port.request_until_match(
      WatchdogCommand::new(Some(Duration::from_millis(1250))),
      Duration::from_millis(2000),
      |resp| match resp {
        WatchdogResponse::Processed => {
          log::info!("🕙 Watchdog timer started");
          Some(true)
        }
        _ => None,
      },
    );

    BootResult {
      mainboard: Mainboard {
        enable_watchdog: false,
        commands_incoming,
        events_outgoing,
        io_port,
        exp_port,
      },
      initial_switch_state: switches,
    }
  }

  pub async fn run(&mut self) {
    loop {
      tokio::select! {
          Some(msg) = self.commands_incoming.recv() => {
            match msg {
              MainboardCommand::Watchdog(enable) => {
                self.enable_watchdog = enable;
              },
              MainboardCommand::SendIo(cmd) => {
                self.io_port.dispatch(&cmd).await;
              },
              MainboardCommand::SendExp(cmd) => {
                self.exp_port.dispatch(&cmd).await;
              }
            }
          }

          // watchdog
          _ = sleep(Duration::from_secs(1)), if self.enable_watchdog => {
            log::trace!("🖥️ -> 👾 : Watchdog tick");
            self.io_port.dispatch(&WatchdogCommand::new(Some(Duration::from_millis(1250)))).await;
          }

          // route incoming messages
          result = self.io_port.read_event() => {
            if let Some(event) = result {
              let event = MainboardIncoming {
                event,
                channel: FastChannel::Io,
              };
              match self.events_outgoing.try_send(event) {
                Ok(_) => {}
                Err(e) => {
                  log::error!("Failed to send mainboard event: {}", e);
                }
              }
            }
          }
      }
    }
  }
}

pub struct BootResult {
  pub mainboard: Mainboard,
  pub initial_switch_state: Vec<SwitchState>,
}

#[derive(Debug, Clone)]
pub struct BootConfig {
  pub io_net_port_path: &'static str,
  pub exp_port_path: &'static str,
  pub platform: FastPlatform,
  pub watchdog_interval: Duration,
}

impl Default for BootConfig {
  fn default() -> Self {
    Self {
      io_net_port_path: "/dev/ttyACM0",
      exp_port_path: "/dev/ttyACM1",
      platform: FastPlatform::Neuron,
      watchdog_interval: Duration::from_millis(1250),
    }
  }
}

#[derive(Debug, Clone)]
pub enum FastPlatform {
  Neuron = 2000,
  RetroSystem11 = 11,
  RetroWPC89 = 89,
  RetroWPC95 = 95,
}

#[derive(Debug, Clone)]
pub enum MainboardCommand {
  Watchdog(bool),
  SendIo(String),
  SendExp(String),
}
