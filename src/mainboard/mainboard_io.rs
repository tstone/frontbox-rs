use std::time::Duration;

use crate::FastChannel;
use crate::mainboard::serial_interface::SerialInterface;
use crate::protocol::configure_hardware::{self, SwitchReporting};
use crate::protocol::{FastResponse, id, watchdog};
use bevy_ecs::event::Event;
use tokio::sync::{broadcast, mpsc};
use tokio::time::sleep;

/// Handles serial
pub struct MainboardIO {
  commands_rx: mpsc::Receiver<MainboardCommand>,
  events_tx: broadcast::Sender<MainboardIncoming>,
  io_port: SerialInterface,
  exp_port: SerialInterface,
  enable_watchdog: bool,
}

#[derive(Debug, Clone, Event)]
pub struct MainboardIncoming {
  pub data: FastResponse,
  pub channel: FastChannel,
}

impl MainboardIO {
  pub async fn boot(
    config: BootConfig,
    commands_rx: mpsc::Receiver<MainboardCommand>,
    events_tx: broadcast::Sender<MainboardIncoming>,
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
      config.switch_reporting.clone(),
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

    MainboardIO {
      enable_watchdog: false,
      commands_rx,
      events_tx,
      io_port,
      exp_port,
    }
  }

  pub async fn run(&mut self) {
    loop {
      tokio::select! {
          Some(msg) = self.commands_rx.recv() => {
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

          // read incoming messages
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
              self.events_tx.send(MainboardIncoming { data: msg, channel: FastChannel::Io }).unwrap();
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
  pub switch_reporting: Option<SwitchReporting>,
}

impl Default for BootConfig {
  fn default() -> Self {
    BootConfig {
      io_net_port_path: "",
      exp_port_path: "",
      platform: FastPlatform::Neuron,
      switch_reporting: Some(SwitchReporting::Verbose),
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
