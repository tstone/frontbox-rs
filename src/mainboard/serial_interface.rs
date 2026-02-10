use std::collections::VecDeque;

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
  event_queue: VecDeque<FastResponse>,
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
      event_queue: VecDeque::new(),
    })
  }

  pub async fn read(&mut self) -> Option<tokio_serial::Result<FastResponse>> {
    // first drain any queued events
    // this can happen when we read a message that isn't a response to a command, but is instead an event (like a switch change)
    if let Some(event) = self.event_queue.pop_front() {
      return Some(Ok(event));
    }

    // otherwise read from the serial port
    self.reader.next().await.map(|result| {
      result
        .map_err(|e| tokio_serial::Error::new(tokio_serial::ErrorKind::Io(e.kind()), e.to_string()))
    })
  }

  // Send off a command without concern for a response
  pub async fn dispatch(&mut self, cmd: &str) {
    if cmd.starts_with("WD:") {
      log::trace!("🖥️ -> 👾 : {}", cmd);
    } else {
      log::debug!("🖥️ -> 👾 : {}", cmd);
    }

    match self.writer.write_all(cmd.as_bytes()).await {
      Ok(_) => (),
      Err(e) => {
        log::error!("Failed to send on {}: {:?}", self.port_name, e);
      }
    }
  }

  /// Send a command and wait for a response to that command
  pub async fn request(&mut self, cmd: &str, timeout: Duration) -> Option<FastResponse> {
    let prefix = Self::extract_prefix(&cmd).to_lowercase();
    self.dispatch(cmd).await;

    tokio::time::timeout(timeout, async {
      loop {
        match self.read().await {
          Some(Ok(response)) => {
            if response.command_prefix().map_or(false, |p| p == prefix) {
              return Some(response);
            } else {
              // If the response doesn't match the prefix, it's likely an event that should be queued for reading by a different process
              self.event_queue.push_back(response);
            }
          }
          Some(Err(e)) => {
            log::error!("Error reading response: {:?}", e);
            return None;
          }
          None => {
            log::error!("Serial stream ended unexpectedly");
            return None;
          }
        }
      }
    })
    .await
    .ok()
    .flatten()
  }

  /// Keep sending the command until a response comes in
  pub async fn request_until_success(
    &mut self,
    cmd: String,
    timeout: Duration,
  ) -> Option<FastResponse> {
    loop {
      if let Some(response) = self.request(&cmd, timeout).await {
        return Some(response);
      }
    }
  }

  // Given a command e.g. `SL@49:10,9F` return the prefix e.g. `SL`
  fn extract_prefix(cmd: &str) -> String {
    cmd.chars().take_while(|&c| c != '@' && c != ':').collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_prefix_io() {
    let cmd = "SL:10,9F";
    let prefix = SerialInterface::extract_prefix(cmd);
    assert_eq!(prefix, "SL");
  }

  #[test]
  fn test_prefix_exp() {
    let cmd = "SL@49:10,9F";
    let prefix = SerialInterface::extract_prefix(cmd);
    assert_eq!(prefix, "SL");
  }
}
