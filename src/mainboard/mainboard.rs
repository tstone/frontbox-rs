use std::time::Duration;

use crate::mainboard::serial_interface::SerialInterface;
use crate::protocol::configure_hardware::{self, SwitchReporting};
use crate::protocol::{FastResponse, id, watchdog};
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
  ) -> Self {
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
    io_port
      .poll_for_response(
        &id::request(),
        Duration::from_millis(250),
        |msg| match msg {
          FastResponse::IdResponse {
            processor,
            product_number,
            firmware_version,
          } => {
            log::info!(
              "🥾 Connected to mainboard {} {} with firmware: {}",
              processor,
              product_number,
              firmware_version
            );
            true
          }
          _ => false,
        },
      )
      .await;

    // configure hardware
    let ch = configure_hardware::request(
      config.platform.clone() as u16,
      Some(SwitchReporting::Verbose),
    );
    log::info!(
      "🥾 Configuring mainboard hardware as platform {:?}",
      config.platform,
    );
    io_port.send(ch.as_bytes()).await;

    // verify watchdog is ready
    io_port
      .poll_for_response(
        watchdog::set(Duration::from_millis(1250)).as_bytes(),
        Duration::from_millis(250),
        |msg| !matches!(msg, FastResponse::Failed(_)),
      )
      .await;
    log::info!("🕙 Watchdog timer started");

    Mainboard {
      enable_watchdog: false,
      commands_incoming,
      events_outgoing,
      io_port,
      exp_port,
    }
  }

  pub async fn run(&mut self) {
    loop {
      tokio::select! {
          Some(msg) = self.commands_incoming.recv() => {
            match msg {
              MainboardCommand::Watchdog(enable) => {
                self.enable_watchdog = enable;
                log::info!("🖥️ Watchdog {}", if enable { "enabled" } else { "disabled" });
              },
              MainboardCommand::SendIo(cmd) => {
                self.io_port.send(cmd.as_bytes()).await;
                log::debug!("🖥️ -> 👾 : {}", cmd);
              },
              MainboardCommand::SendExp(cmd) => {
                self.exp_port.send(cmd.as_bytes()).await;
                log::debug!("🖥️ -> 👾 : {}", cmd);
              }
            }
          }

          // watchdog
          _ = sleep(Duration::from_secs(1)), if self.enable_watchdog => {
            log::trace!("🖥️ -> 👾 : Watchdog tick");
            self.io_port.send(watchdog::set(Duration::from_millis(1250)).as_bytes()).await;
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
