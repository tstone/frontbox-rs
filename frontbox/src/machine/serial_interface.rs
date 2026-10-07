use std::collections::VecDeque;
use std::time::Duration;

use fast_protocol::FastBinaryDispatch;
use fast_protocol::FastQuery;
use fast_protocol::ProcessedResponse;
use futures_util::StreamExt;
use tokio::io::{AsyncWriteExt, ReadHalf, WriteHalf};
use tokio_serial::{DataBits, FlowControl, Parity, SerialStream, StopBits};
use tokio_util::codec::FramedRead;

use crate::machine::fast_codec::FastRawCodec;
use fast_protocol::FastCommand;
use fast_protocol::RawResponse;
use fast_protocol::{EventResponse, FastResponseError};

const BAUD_RATE: u32 = 921_600;

pub struct SerialInterface {
  port_name: String,
  reader: FramedRead<ReadHalf<SerialStream>, FastRawCodec>,
  writer: WriteHalf<SerialStream>,
  // if events are encountered while attempting to read a response, they are queued here for later retrieval
  event_queue: VecDeque<EventResponse>,
  // if a response is encountered that is not the expected response, it is queued here for later retrieval
  response_queue: VecDeque<QueuedResponse>,
  response_ttl: Duration,
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
      response_queue: VecDeque::new(),
      event_queue: VecDeque::new(),
      response_ttl: Duration::from_secs(4), // after this time unclaimed messages fall out of the queue
    })
  }

  pub async fn read_event(&mut self) -> Option<EventResponse> {
    // First check if there is an event already in the queue. This may have happened by a prior call to
    // query or command that attempted to get a response but actually received an event
    if let Some(event) = self.event_queue.pop_front() {
      return Some(event);
    }

    // If none queued, read from the serial port until an event arrives
    loop {
      match self.read_from_port().await {
        Some(Ok(raw)) => {
          if let Some(event) = EventResponse::parse(&raw).ok() {
            return Some(event);
          } else {
            self.response_queue.push_back(QueuedResponse {
              raw,
              received_at: std::time::Instant::now(),
            });
          }
        }
        Some(Err(e)) => {
          log::error!(target: "frontbox::serial", "Serial read error: {}", e);
          return None;
        }
        None => return None,
      }
    }
  }

  pub fn take_queued_events(&mut self) -> impl Iterator<Item = EventResponse> + '_ {
    self.event_queue.drain(..)
  }

  /// Responses in the queue can be one of three things: (a) an event from hardware, e.g. switch hit, (b) a response to a command that was waiting for a response,
  /// or (c) some part of a response that is neither. Items of C fill up the queue over time. If a prefix shows up frequently here, it might need to be sent as
  /// request instead of dispatch.
  fn prune_queue(&mut self, prefix: &str) {
    let now = std::time::Instant::now();
    self.response_queue.retain(|r| {
      // drop any responses that match the prefix, since they are now being claimed
      if r.raw.prefix.to_lowercase() == prefix {
        return false;
      }

      let expired = now.duration_since(r.received_at) > self.response_ttl;
      if expired {
        log::trace!(
          target: "frontbox::serial",
          "Expiring unclaimed queue response: {}:{}",
          r.raw.prefix,
          r.raw.payload
        );
      }
      !expired
    });
  }

  async fn read_from_port(&mut self) -> Option<tokio_serial::Result<RawResponse>> {
    let resp = {
      self.reader.next().await.map(|result| {
        result.map_err(|e| {
          tokio_serial::Error::new(tokio_serial::ErrorKind::Io(e.kind()), e.to_string())
        })
      })
    };

    match &resp {
      Some(Ok(raw)) if raw.prefix == "WD" => {
        log::trace!(target: "frontbox::serial", "👾 -> 🖥️ : {}:{}", raw.prefix, raw.payload)
      }
      Some(Ok(raw)) => {
        log::debug!(target: "frontbox::serial", "👾 -> 🖥️ : {}:{}", raw.prefix, raw.payload)
      }
      _ => {}
    }

    resp
  }

  // Send off a command without concern for a response
  async fn send(&mut self, cmd: &[u8]) {
    if cmd.starts_with(b"WD:") || cmd.starts_with(b"R") {
      log::trace!(target: "frontbox::serial", "🖥️ -> 👾 : {}", String::from_utf8_lossy(cmd));
    } else {
      log::debug!(target: "frontbox::serial", "🖥️ -> 👾 : {}", String::from_utf8_lossy(cmd));
    }

    match self.writer.write_all(cmd).await {
      Ok(_) => (),
      Err(e) => {
        log::error!(target: "frontbox::serial", "Failed to send on {}: {:?}", self.port_name, e);
      }
    }
  }

  pub async fn dispatch<C: FastBinaryDispatch + ?Sized>(&mut self, cmd: &C) {
    self.send(&cmd.to_bytes()).await
  }

  async fn query_inner<R>(
    &mut self,
    prefix: &str,
    timeout: Duration,
    parse: impl Fn(RawResponse) -> Result<R, FastResponseError>,
  ) -> Result<R, FastResponseError> {
    let prefix = prefix.to_lowercase();
    self.prune_queue(&prefix);

    if let Some(pos) = self
      .response_queue
      .iter()
      .position(|r| r.raw.prefix.to_lowercase() == prefix)
    {
      let response = self.response_queue.remove(pos).unwrap();
      return parse(response.raw);
    }

    tokio::time::timeout(timeout, async {
      loop {
        match self.read_from_port().await {
          Some(Ok(response)) => {
            if response.prefix.to_lowercase() == prefix {
              return parse(response);
            } else if let Some(event) = EventResponse::parse(&response).ok() {
              self.event_queue.push_back(event);
            } else {
              self.response_queue.push_back(QueuedResponse {
                raw: response,
                received_at: std::time::Instant::now(),
              });
            }
          }
          Some(Err(e)) => {
            log::error!(target: "frontbox::serial", "Error reading response: {:?}", e);
            return Err(FastResponseError::UnknownResponse);
          }
          None => {
            log::error!(target: "frontbox::serial", "Serial stream ended unexpectedly");
            return Err(FastResponseError::UnknownResponse);
          }
        }
      }
    })
    .await
    .unwrap_or_else(|_| Err(FastResponseError::Timeout))
  }

  /// Dispatch a command and wait for a response
  pub async fn query<C: FastQuery>(
    &mut self,
    cmd: &C,
    timeout: Duration,
  ) -> Result<C::Response, FastResponseError> {
    self.dispatch(cmd).await;
    self
      .query_inner(cmd.prefix(), timeout, |raw| cmd.parse(raw))
      .await
  }

  pub async fn command<C: FastCommand + ?Sized>(
    &mut self,
    cmd: &C,
    timeout: Duration,
  ) -> Result<ProcessedResponse, FastResponseError> {
    self.dispatch(cmd).await;
    self
      .query_inner(cmd.prefix(), timeout, |raw| cmd.parse(raw))
      .await
  }
}

struct QueuedResponse {
  raw: RawResponse,
  received_at: std::time::Instant,
}
