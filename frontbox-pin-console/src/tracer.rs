use frontbox::prelude::{Hardware, app_tracer::*};
use std::net::SocketAddr;
use tokio::sync::mpsc;

use crate::console_hub::ConsoleHub;
use crate::console_plane::ConsolePlane;
use crate::server;

pub const DEFAULT_ADDR: ([u8; 4], u16) = ([0, 0, 0, 0], 3000);

/// Web-based dashboard for visualizing Frontbox state.
///
/// ```rust,ignore
/// app.tracer(
///   WebTracer::new()
///     .port(3100)
///     .plane(ConsolePlane::new("Playfield", &PLAYFIELD).image("art/playfield.png")),
/// );
/// ```
///
/// Nothing starts until the app boots, so a `WebTracer` can be built outside of a tokio runtime.
pub struct WebTracer {
  tx: mpsc::UnboundedSender<TraceEvent>,
  /// Taken when the console starts in `init`; events sent before then wait in the channel
  rx: Option<mpsc::UnboundedReceiver<TraceEvent>>,
  hub: ConsoleHub,
  addr: SocketAddr,
  planes: Vec<ConsolePlane>,
}

impl WebTracer {
  pub fn new() -> Self {
    let (tx, rx) = mpsc::unbounded_channel::<TraceEvent>();
    Self {
      tx,
      rx: Some(rx),
      hub: ConsoleHub::new(),
      addr: DEFAULT_ADDR.into(),
      planes: Vec::new(),
    }
  }

  /// Address to serve the console on. Defaults to `0.0.0.0:3000`.
  pub fn addr(mut self, addr: impl Into<SocketAddr>) -> Self {
    self.addr = addr.into();
    self
  }

  /// Port to serve the console on, keeping the address
  pub fn port(mut self, port: u16) -> Self {
    self.addr.set_port(port);
    self
  }

  /// Add a surface of the machine for the console to draw
  pub fn plane(mut self, plane: ConsolePlane) -> Self {
    self.planes.push(plane);
    self
  }

  pub fn planes(&self) -> &[ConsolePlane] {
    &self.planes
  }

  /// Start receiving trace events and serving the console
  fn start(&mut self) {
    let Some(mut rx) = self.rx.take() else {
      return;
    };

    // ConsoleHub is mainly just a handle to the Arc of Hub state
    // Create two copies, one to receive incoming events from Frontbox
    let recv_hub = self.hub.clone();
    tokio::spawn(async move {
      while let Some(event) = rx.recv().await {
        recv_hub.ingest(event);
      }
      log::info!(target: "frontbox_pin_console", "Trace event channel closed");
    });
    // And a second to handle web interactions
    tokio::spawn(server::serve(self.hub.clone(), self.addr));
  }
}

impl Default for WebTracer {
  fn default() -> Self {
    Self::new()
  }
}

impl AppTracer for WebTracer {
  fn init(&mut self, hardware: &Hardware) {
    // the app calls this at boot, inside its runtime
    self.start();
    self.hub.set_hardware(hardware.clone());
  }

  fn sender(&self) -> mpsc::UnboundedSender<TraceEvent> {
    self.tx.clone()
  }
}
