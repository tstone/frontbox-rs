use std::collections::VecDeque;
use std::time::Duration;

use futures_util::StreamExt;
use tokio::io::{AsyncWriteExt, ReadHalf, WriteHalf};
use tokio_serial::{
  DataBits, FlowControl, Parity, SerialPort, SerialPortBuilderExt, SerialStream, StopBits,
};
use tokio_util::codec::FramedRead;

use crate::machine::fast_codec::FastRawCodec;
use crate::protocol::fast_command::FastCommand;
use crate::protocol::raw_response::RawResponse;
use crate::protocol::{EventResponse, FastResponseError};

const BAUD_RATE: u32 = 921_600;

pub struct SerialInterface {
  port_name: String,
  reader: FramedRead<ReadHalf<SerialStream>, FastRawCodec>,
  writer: WriteHalf<SerialStream>,
  event_queue: VecDeque<RawResponse>,
}

impl SerialInterface {
  pub async fn new(port_path: &str) -> tokio_serial::Result<Self> {
    let port = tokio_serial::new(port_path, BAUD_RATE)
      .data_bits(DataBits::Eight)
      .parity(Parity::None)
      .stop_bits(StopBits::One)
      .flow_control(FlowControl::None);

    let port = SerialStream::open(&port)?;

    let (reader, mut writer) = tokio::io::split(port);

    // before this port starts reading, send a bunch of carriage returns to clear out any junk in the buffer.
    // https://fastpinball.com/programming/framework/exp/#clear-out-the-serial-buffer
    writer.write_all("\r\r\r\r".as_bytes()).await?;

    let mut framed_reader = FramedRead::new(reader, FastRawCodec::new());

    // poll reader until there is no unexpected messages
    // this also clears out anything that was from a prior run
    log::trace!("Draining serial buffer on {} before continuing", port_path);
    let drain_timeout = Duration::from_millis(300); // Adjust as needed
    loop {
      match tokio::time::timeout(drain_timeout, framed_reader.next()).await {
        Ok(Some(Ok(_))) => continue,
        _ => break,
      }
    }

    Ok(SerialInterface {
      port_name: port_path.to_string(),
      reader: framed_reader,
      writer,
      event_queue: VecDeque::new(),
    })
  }

  pub async fn read_event(&mut self) -> Option<EventResponse> {
    match self.read().await {
      Some(Ok(raw)) => EventResponse::parse(raw).ok(),
      _ => None,
    }
  }

  async fn read(&mut self) -> Option<tokio_serial::Result<RawResponse>> {
    let resp = {
      // first drain any queued events
      // this can happen when we read a message that isn't a response to a command, but is instead an event (like a switch change)
      if let Some(event) = self.event_queue.pop_front() {
        return Some(Ok(event));
      }

      // otherwise read from the serial port
      self.reader.next().await.map(|result| {
        result.map_err(|e| {
          tokio_serial::Error::new(tokio_serial::ErrorKind::Io(e.kind()), e.to_string())
        })
      })
    };

    match &resp {
      Some(Ok(raw)) if raw.prefix == "WD" => {
        log::trace!("👾 -> 🖥️ : {}:{}", raw.prefix, raw.payload)
      }
      Some(Ok(raw)) => {
        log::debug!("👾 -> 🖥️ : {}:{}", raw.prefix, raw.payload)
      }
      _ => {}
    }

    resp
  }

  // Send off a command without concern for a response
  async fn send(&mut self, cmd: &str) {
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

  pub async fn dispatch<C: FastCommand>(&mut self, cmd: &C) {
    self.send(&cmd.to_string()).await
  }

  /// Send a command and wait for a response to that command
  pub async fn request<C: FastCommand>(
    &mut self,
    cmd: &C,
    timeout: Duration,
  ) -> Result<C::Response, FastResponseError> {
    self.dispatch(cmd).await;

    tokio::time::timeout(timeout, async {
      loop {
        match self.read().await {
          Some(Ok(response)) => {
            if response.prefix.to_lowercase() == C::prefix() {
              return cmd.parse(response);
            } else {
              // If the response doesn't match the prefix, it's likely an event that should be queued for reading by a different process
              self.event_queue.push_back(response);
            }
          }
          Some(Err(e)) => {
            log::error!("Error reading response: {:?}", e);
            return Err(FastResponseError::UnknownResponse); // ???
          }
          None => {
            log::error!("Serial stream ended unexpectedly");
            return Err(FastResponseError::UnknownResponse);
          }
        }
      }
    })
    .await
    .unwrap_or_else(|_| Err(FastResponseError::Timeout))
  }

  /// Keep sending the command until a response comes in
  pub async fn request_until_match<C: FastCommand, R>(
    &mut self,
    cmd: C,
    timeout: Duration,
    f: fn(C::Response) -> Option<R>,
  ) -> R {
    loop {
      if let Ok(response) = self.request(&cmd, timeout).await {
        if let Some(result) = f(response) {
          return result;
        }
      }

      // sleep if a match wasn't found
      tokio::time::sleep(timeout).await;
    }
  }
}
