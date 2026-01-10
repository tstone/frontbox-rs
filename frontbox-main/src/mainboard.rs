use std::time::Duration;

use frontbox_fast::FastResponse;
use frontbox_fast::protocol::configure_hardware::{self, SwitchReporting};
use frontbox_fast::protocol::{id, watchdog};
use tokio::sync::mpsc;
use tokio::time::sleep;

use crate::serial_interface::SerialInterface;

pub struct Mainboard {
  config: MainboardConfig,
  command_tx: Option<mpsc::Sender<String>>,
}

impl Mainboard {
  pub fn new(config: MainboardConfig) -> Self {
    Mainboard {
      config,
      command_tx: None,
    }
  }

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
              "🥾 Connected to mainboard: processor={}, product_number={}, firmware_version={}",
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
        watchdog::set(Some(1250)).as_bytes(),
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
    // let mut exp_port = SerialInterface::new(self.config.exp_port_path)
    //   .await
    //   .expect("Failed to open EXP port");
    // log::info!("🥾 Opened EXP port at {}", self.config.exp_port_path);

    let (command_tx, mut command_rx) = mpsc::channel::<String>(32);
    self.command_tx = Some(command_tx);

    // start system loop
    loop {
      tokio::select! {
          // watchdog
          _ = sleep(Duration::from_secs(1)) => {
            log::trace!("🖥️ -> 👾 : Watchdog tick");
            io_port.send(watchdog::set(Some(1250)).as_bytes()).await;
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

          // TODO: run game logic

          // write outgoing messages
          Some(cmd) = command_rx.recv() => {
            io_port.send(cmd.as_bytes()).await;
            log::debug!("🖥️ -> 👾 : {}", cmd);
          }
      }
    }
  }
}

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
