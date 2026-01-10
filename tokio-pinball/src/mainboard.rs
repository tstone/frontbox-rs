use fast_pinball_protocol::FastResponse;
use fast_pinball_protocol::protocol::configure_hardware::{self, SwitchReporting};
use fast_pinball_protocol::protocol::id;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};
use tokio_util::codec::FramedRead;

use crate::FastCodec;

const BAUD_RATE: u32 = 921_600;

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

  fn open_port(port_path: &str) -> tokio_serial::Result<tokio_serial::SerialStream> {
    let port = tokio_serial::new(port_path, BAUD_RATE)
      .data_bits(tokio_serial::DataBits::Eight)
      .parity(tokio_serial::Parity::None)
      .stop_bits(tokio_serial::StopBits::One)
      .flow_control(tokio_serial::FlowControl::None);

    tokio_serial::SerialStream::open(&port)
  }

  pub async fn run(&mut self) {
    // open IO port
    let io_port =
      Mainboard::open_port(self.config.io_net_port_path).expect("Failed to open IO port");
    let (io_reader, mut io_writer) = tokio::io::split(io_port);
    let mut lines = FramedRead::new(io_reader, FastCodec::new());

    // boot sequence
    loop {
      io_writer.write_all(id::request()).await.unwrap();
      let timeout = time::timeout(Duration::from_millis(250), lines.next());
      match timeout.await {
        Ok(Some(Ok(line))) => match id::response(line) {
          Ok(FastResponse::IdResponse {
            processor,
            product_number,
            firmware_version,
          }) => {
            log::info!(
              "Mainboard detected: processor={}, product_number={}, firmware_version={}",
              processor,
              product_number,
              firmware_version
            );
            break;
          }
          _ => {
            log::trace!("No response to ID request, retrying...");
          }
        },
        _ => {
          log::trace!("No response to ID request, retrying...");
        }
      }
    }

    // configure hardware
    let ch = configure_hardware::request(
      self.config.platform.clone() as u16,
      self.config.switch_reporting.clone(),
    );
    io_writer.write_all(ch.as_bytes()).await.unwrap();

    // open EXP port
    // let exp_port =
    //   Mainboard::open_port(self.config.exp_port_path).expect("Failed to open EXP port");

    let (command_tx, mut command_rx) = mpsc::channel::<String>(32);
    self.command_tx = Some(command_tx);

    // start system loop
    loop {
      tokio::select! {
          // TODO: watchdog

          Some(Ok(line)) = lines.next() => {
            log::debug!("👾 -> 🖥️: {}", line);
          }

          Some(cmd) = command_rx.recv() => {
            io_writer.write_all(cmd.as_bytes()).await.unwrap();
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
      io_net_port_path: "/dev/ttyACM0",
      exp_port_path: "/dev/ttyACM1",
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
