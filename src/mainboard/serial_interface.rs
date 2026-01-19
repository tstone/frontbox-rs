use futures_util::StreamExt;
use tokio::io::{AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::time::{self, Duration};
use tokio_serial::*;
use tokio_util::codec::FramedRead;

use crate::FastCodec;
use crate::protocol::FastResponse;

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
