use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Instant;

use frontbox_core::connection_retry_wait;
use image::Rgb;

use crate::DisplayDevice;

/// Writes the frames [`DisplaySystem`](crate::DisplaySystem) sends to the device, on a thread of its own so a slow or stalled
/// device never holds up the run loop. Only the newest frame matters: any that arrive while one is being written are
/// skipped. It opens the device, retrying with a growing wait until it can, and opens it again whenever a write fails.
pub(crate) struct DisplayManager {
  /// The display's name, for logs
  name: String,
  device: Box<dyn DisplayDevice>,
  frames: Receiver<Vec<Rgb<u8>>>,
  /// The newest frame, kept to show again on a new connection
  frame: Option<Vec<Rgb<u8>>>,
  /// Whether `frame` hasn't been written to the current connection
  unwritten: bool,
  connection: Connection,
}

impl DisplayManager {
  pub fn new(name: String, device: Box<dyn DisplayDevice>, frames: Receiver<Vec<Rgb<u8>>>) -> Self {
    Self {
      name,
      device,
      frames,
      frame: None,
      unwritten: false,
      connection: Connection::Waiting {
        retry_at: Instant::now(),
        failures: 0,
      },
    }
  }

  /// Runs until the [`DisplaySystem`](crate::DisplaySystem) is dropped, after writing the frames it sent before then
  pub fn run(mut self) {
    loop {
      let received = match self.connection {
        Connection::Connected => self.frames.recv().map_err(|_| RecvTimeoutError::Disconnected),
        Connection::Waiting { retry_at, .. } => self
          .frames
          .recv_timeout(retry_at.saturating_duration_since(Instant::now())),
      };
      match received {
        Ok(frame) => {
          self.frame = Some(self.frames.try_iter().last().unwrap_or(frame));
          self.unwritten = true;
        }
        Err(RecvTimeoutError::Timeout) => {}
        Err(RecvTimeoutError::Disconnected) => break,
      }

      if let Connection::Waiting { retry_at, failures } = self.connection
        && Instant::now() >= retry_at
      {
        self.connect(failures);
      }
      if matches!(self.connection, Connection::Connected) {
        self.write();
      }
    }
  }

  fn connect(&mut self, failures: u32) {
    match self.device.open() {
      Ok(()) => {
        log::info!(target: "frontbox::display", "🖥️ Opened {}", self.name);
        self.connection = Connection::Connected;
        self.unwritten = self.frame.is_some();
      }
      Err(reason) => {
        let failures = failures + 1;
        let wait = connection_retry_wait(failures);
        // the first failure is worth knowing about, then once in a while so a device that never turns up isn't silent
        if failures == 1 || failures.is_multiple_of(12) {
          log::warn!(
            target: "frontbox::display",
            "🖥️ Couldn't open {} ({}), attempt {}. Retrying in {:?}",
            self.name,
            reason,
            failures,
            wait
          );
        } else {
          log::debug!(
            target: "frontbox::display",
            "🖥️ Couldn't open {} ({}), attempt {}. Retrying in {:?}",
            self.name,
            reason,
            failures,
            wait
          );
        }
        self.connection = Connection::Waiting {
          retry_at: Instant::now() + wait,
          failures,
        };
      }
    }
  }

  fn write(&mut self) {
    let Some(frame) = self.frame.as_ref().filter(|_| self.unwritten) else {
      return;
    };
    match self.device.write(frame) {
      Ok(()) => self.unwritten = false,
      Err(reason) => {
        log::warn!(target: "frontbox::display", "🖥️ Lost {} ({}), reconnecting", self.name, reason);
        self.connection = Connection::Waiting {
          retry_at: Instant::now(),
          failures: 0,
        };
      }
    }
  }
}

/// Where the [`DisplayManager`] is with the device
#[derive(Clone, Copy)]
enum Connection {
  Connected,
  /// The last attempt to open it failed, and the next starts at `retry_at`
  Waiting { retry_at: Instant, failures: u32 },
}
