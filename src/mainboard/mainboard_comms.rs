use std::time::Duration;

use crate::mainboard::serial_interface::SerialInterface;
use crate::protocol::configure_hardware::{self, SwitchReporting};
use crate::protocol::{FastResponse, id, watchdog};
use tokio::sync::mpsc;
use tokio::time::sleep;

pub struct MainboardComms {
  config: MainboardConfig,
  bridge_rx: mpsc::Receiver<MainboardCommand>,
  io_tx: Option<mpsc::Sender<String>>,
  exp_tx: Option<mpsc::Sender<String>>,
  enable_watchdog: bool,
}

impl MainboardComms {
  pub fn new(config: MainboardConfig, bridge_rx: mpsc::Receiver<MainboardCommand>) -> Self {
    MainboardComms {
      config,
      io_tx: None,
      exp_tx: None,
      enable_watchdog: false,
      bridge_rx,
    }
  }

  pub fn send_io(&self, cmd: String) {
    if let Some(tx) = &self.io_tx {
      let _ = tx.try_send(cmd);
    }
  }

  pub fn send_exp(&self, cmd: String) {
    if let Some(tx) = &self.exp_tx {
      let _ = tx.try_send(cmd);
    }
  }

  // how to let subscriptions (e.g. switch events)

  async fn initialize_io_port(&mut self, io_port: &mut SerialInterface) {
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
      self.config.platform.clone() as u16,
      self.config.switch_reporting.clone(),
    );
    log::info!(
      "🥾 Configuring mainboard hardware as platform {:?}",
      self.config.platform,
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
  }

  pub async fn run(&mut self) {
    // open IO port
    let mut io_port = SerialInterface::new(self.config.io_net_port_path)
      .await
      .expect("Failed to open IO NET port");

    log::info!("🥾 Opened IO NET port at {}", self.config.io_net_port_path);
    self.initialize_io_port(&mut io_port).await;

    // open EXP port
    let mut exp_port = SerialInterface::new(self.config.exp_port_path)
      .await
      .expect("Failed to open EXP port");
    log::info!("🥾 Opened EXP port at {}", self.config.exp_port_path);

    let (io_tx, mut io_rx) = mpsc::channel::<String>(32);
    self.io_tx = Some(io_tx);

    let (exp_tx, mut exp_rx) = mpsc::channel::<String>(32);
    self.exp_tx = Some(exp_tx);

    // start system loop
    loop {
      tokio::select! {
          Some(msg) = self.bridge_rx.recv() => {
            match msg {
              MainboardCommand::Watchdog(enable) => {
                self.enable_watchdog = enable;
                log::info!("🖥️ Watchdog {}", if enable { "enabled" } else { "disabled" });
              }
            }
          }

          // watchdog
          _ = sleep(Duration::from_secs(1)), if self.enable_watchdog => {
            log::trace!("🖥️ -> 👾 : Watchdog tick");
            io_port.send(watchdog::set(Duration::from_millis(1250)).as_bytes()).await;
          }

          // read incoming messages
          result = io_port.read() => {
            if let Some(Ok(msg)) = result {
              match msg {
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
            }
          }

          // write outgoing messages
          Some(cmd) = io_rx.recv() => {
            io_port.send(cmd.as_bytes()).await;
            log::debug!("🖥️ -> 👾 : {}", cmd);
          }

          Some(cmd) = exp_rx.recv() => {
            exp_port.send(cmd.as_bytes()).await;
            log::debug!("🖥️ -> 👾 : {}", cmd);
          }
      }
    }
  }
}

#[derive(Debug, Clone)]
pub struct MainboardConfig {
  pub io_net_port_path: &'static str,
  pub exp_port_path: &'static str,
  pub platform: FastPlatform,
  pub switch_reporting: Option<SwitchReporting>,
}

impl Default for MainboardConfig {
  fn default() -> Self {
    MainboardConfig {
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
}
