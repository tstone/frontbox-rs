use fast_pinball_protocol::FastResponse;
use fast_pinball_protocol::protocol::configure_hardware::{self, SwitchReporting};
use fast_pinball_protocol::protocol::{id, watchdog};
use futures_util::StreamExt;
use tokio::io::{AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::sync::mpsc;
use tokio::time::{self, Duration, sleep};
use tokio_serial::*;
use tokio_util::codec::FramedRead;

use crate::FastCodec;

const BAUD_RATE: u32 = 921_600;

pub struct SerialInterface {
  port_name: String,
  reader: FramedRead<ReadHalf<SerialStream>, FastCodec>,
  writer: WriteHalf<SerialStream>,
}

impl SerialInterface {
  pub async fn new(port_path: &str) -> tokio_serial::Result<Self> {
    // let port = Mainboard::open_port(port_path)?;
    let port = tokio_serial::new(port_path, BAUD_RATE)
      .data_bits(DataBits::Eight)
      .parity(Parity::None)
      .stop_bits(StopBits::One)
      .flow_control(FlowControl::None);

    let port = SerialStream::open(&port)?;
    let (reader, writer) = tokio::io::split(port);
    let framed_reader = FramedRead::new(reader, FastCodec::new());

    Ok(SerialInterface {
      port_name: port_path.to_string(),
      reader: framed_reader,
      writer,
    })
  }

  pub async fn read(&mut self) -> Option<tokio_serial::Result<FastResponse>> {
    self.reader.next().await.map(|result| {
      result
        .map_err(|e| tokio_serial::Error::new(tokio_serial::ErrorKind::Io(e.kind()), e.to_string()))
    })
  }

  pub async fn send(&mut self, cmd: &[u8]) {
    match self.writer.write_all(cmd).await {
      Ok(_) => (),
      Err(e) => {
        log::error!("Failed to send on {}: {:?}", self.port_name, e);
      }
    }
  }

  pub async fn poll_for_response(
    &mut self,
    cmd: &[u8],
    timeout_duration: Duration,
    predicate: fn(FastResponse) -> bool,
  ) {
    loop {
      self.send(cmd).await;

      let timeout = time::timeout(timeout_duration, self.reader.next());
      match timeout.await {
        Ok(Some(Ok(msg))) => {
          if predicate(msg.clone()) {
            break;
          }
        }
        Ok(Some(Err(e))) => {
          log::error!("Error waiting for response: {:?}", e);
        }
        _ => (),
      }
    }
  }
}

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

  pub async fn run(&mut self) {
    // open IO port
    let mut io_port = SerialInterface::new(self.config.io_net_port_path)
      .await
      .expect("Failed to open IO NET port");
    log::info!("Opened IO NET port at {}", self.config.io_net_port_path);

    // boot sequence
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
              "Connected to mainboard: processor={}, product_number={}, firmware_version={}",
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
      "Configuring mainboard hardware as platform {:?} with switch verbosity {:?}",
      self.config.platform,
      self.config.switch_reporting
    );
    io_port.send(ch.as_bytes()).await;

    // TODO: open EXP port

    // verify watchdog is running
    io_port
      .poll_for_response(
        watchdog::set(Some(1250)).as_bytes(),
        Duration::from_millis(250),
        |msg| !matches!(msg, FastResponse::Failed(_)),
      )
      .await;
    log::info!("Watchdog timer started");

    let (command_tx, mut command_rx) = mpsc::channel::<String>(32);
    self.command_tx = Some(command_tx);

    // start system loop
    loop {
      tokio::select! {
          // watchdog
          _ = sleep(Duration::from_secs(1)) => {
            log::trace!("Watchdog tick");
            io_port.send(watchdog::set(Some(1250)).as_bytes()).await;
          }

          // read incoming messages
          result = io_port.read() => {
            if let Some(Ok(line)) = result {
              if matches!(line, FastResponse::Failed(_)) || matches!(line, FastResponse::Invalid(_)) {
                log::warn!("Received error from mainboard: {:?}", line);
              } else if matches!(line, FastResponse::WatchdogProcessed) {
                log::trace!("Received watchdog ack from mainboard");
              } else {
                log::debug!("👾 -> 🖥️: {:?}", line);
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
