use std::time::Duration;

use crate::mainboard::serial_interface::SerialInterface;
use crate::protocol::configure_hardware::{self, SwitchReporting};
use crate::protocol::{FastResponse, SwitchState, id, report_switches, watchdog};
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
  pub data: FastResponse,
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
    loop {
      let resp = io_port
        .request(id::request(), Duration::from_millis(2000))
        .await;
      match resp {
        Some(FastResponse::IdResponse {
          processor,
          product_number,
          firmware_version,
        }) => {
          log::info!(
            "🥾 Connected to mainboard {} {} with firmware: {}",
            processor,
            product_number,
            firmware_version
          );
          break;
        }
        _ => {}
      };
    }

    // configure hardware (instruct mainboard which 'platform' it is)
    log::info!(
      "🥾 Configuring mainboard hardware as platform {:?}",
      config.platform,
    );
    let _ = io_port
      .request(
        &configure_hardware::request(
          config.platform.clone() as u16,
          Some(SwitchReporting::Verbose),
        ),
        Duration::from_millis(2000),
      )
      .await;

    let switches = io_port
      .request(&report_switches::request(), Duration::from_millis(2000))
      .await
      .expect("Failed to get initial switch states from mainboard");

    // verify watchdog is ready
    loop {
      let resp = io_port
        .request(
          &watchdog::set(Duration::from_millis(1250)),
          Duration::from_millis(2000),
        )
        .await;
      match resp {
        Some(FastResponse::Processed { .. }) => {
          log::info!("🕙 Watchdog timer started");
          break;
        }
        _ => {}
      };
    }

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
            self.io_port.dispatch(&watchdog::set(Duration::from_millis(1250))).await;
          }

          // route incoming messages
          result = self.io_port.read() => {
            if let Some(Ok(msg)) = result {
              match msg.clone() {
                FastResponse::Processed(cmd) => {
                  log::trace!("👾 -> 🖥️ : Processed {} ", cmd);
                }
                FastResponse::Failed(cmd) => {
                  log::warn!("👾 -> 🖥️ : ⚠️ Failed {}", cmd);
                }
                FastResponse::Invalid(cmd) => {
                  log::warn!("👾 -> 🖥️ : ⚠️ Invalid {}", cmd);
                }
                _ => {
                  log::debug!("👾 -> 🖥️: {:?}", msg);
                }
              }

              let event = MainboardIncoming {
                data: msg,
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
