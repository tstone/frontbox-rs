use frontbox::prelude::{Hardware, ReferencePlane, app_tracer::*};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::console_hub::ConsoleHub;
use crate::planes::IntoTracedPlane;
use crate::server;

pub const DEFAULT_ADDR: ([u8; 4], u16) = ([0, 0, 0, 0], 3000);

/// How often changed LED colors are sent to the console (20 times a second)
const LED_FLUSH_INTERVAL: Duration = Duration::from_millis(50);

/// Web-based dashboard for visualizing Frontbox state.
///
/// ```rust,ignore
/// app.tracer(
///   WebTracer::new()
///     .port(3100)
///     .plane(&planes::PLAYFIELD)
///     .plane(plane_path!(planes::BACKBOX)),
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
  /// Each plane, and how the machine's code refers to it
  planes: Vec<(&'static ReferencePlane, Option<&'static str>)>,
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

  /// Add a surface of the machine for the console to draw, with its image if it has one. Pass
  /// [`plane_path!`](crate::plane_path)`(planes::PLAYFIELD)` instead of `&planes::PLAYFIELD` to let the console copy
  /// positions on it as code.
  pub fn plane(mut self, plane: impl IntoTracedPlane) -> Self {
    self.planes.push(plane.into_traced_plane());
    self
  }

  pub fn planes(&self) -> &[(&'static ReferencePlane, Option<&'static str>)] {
    &self.planes
  }

  /// Start receiving trace events and serving the console
  fn start(&mut self) {
    let Some(mut rx) = self.rx.take() else {
      return;
    };
    self.hub.set_planes(&self.planes);

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

    // LED colors go out together at a steady rate rather than as each batch arrives
    let led_hub = self.hub.clone();
    tokio::spawn(async move {
      let mut interval = tokio::time::interval(LED_FLUSH_INTERVAL);
      interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
      loop {
        interval.tick().await;
        led_hub.flush_leds();
      }
    });
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
