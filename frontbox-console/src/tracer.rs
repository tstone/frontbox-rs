use frontbox_core::prelude::{AppConfig, Hardware, app_tracer::*};
use frontbox_metrics::MetricsDb;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::console_hub::ConsoleHub;
use crate::server;

pub const DEFAULT_ADDR: ([u8; 4], u16) = ([0, 0, 0, 0], 3000);

const LED_FLUSH_INTERVAL: Duration = Duration::from_millis(50);

/// Web-based dashboard for visualizing Frontbox state.
///
/// ```rust,ignore
/// app.tracer(
///   WebTracer::new()
///     .port(3100)
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
}

impl WebTracer {
  pub fn new() -> Self {
    let (tx, rx) = mpsc::unbounded_channel::<TraceEvent>();
    Self {
      tx,
      rx: Some(rx),
      hub: ConsoleHub::new(),
      addr: DEFAULT_ADDR.into(),
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

  fn start(&mut self) {
    let Some(mut rx) = self.rx.take() else {
      return;
    };
    // ConsoleHub is a cheap handle to shared state; each task gets its own clone
    let recv_hub = self.hub.clone();
    tokio::spawn(async move {
      while let Some(event) = rx.recv().await {
        recv_hub.ingest(event);
      }
      log::info!(target: "frontbox_core::console", "Trace event channel closed");
    });
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
  fn init(
    &mut self,
    hardware: &Hardware,
    config: &AppConfig,
    control: mpsc::UnboundedSender<TracerControlEvent>,
    metrics: Option<MetricsDb>,
  ) {
    // the app calls this at boot, inside its runtime
    self.hub.set_control(control);
    self.hub.set_metrics(metrics);
    self.hub.set_led_radius(config.led_radius);
    self.hub.set_content(config.content_path.clone());
    self.hub.set_planes(&config.planes);
    self.start();
    self.hub.set_hardware(hardware.clone());
  }

  fn sender(&self) -> mpsc::UnboundedSender<TraceEvent> {
    self.tx.clone()
  }
}
