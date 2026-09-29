use frontbox::prelude::{Hardware, app_tracer::*};
use std::net::SocketAddr;
use tokio::sync::mpsc;

use crate::console_hub::ConsoleHub;
use crate::server;

pub const DEFAULT_ADDR: ([u8; 4], u16) = ([0, 0, 0, 0], 3000);

pub struct WebTracer {
  tx: mpsc::UnboundedSender<TraceEvent>,
  hub: ConsoleHub,
}

impl WebTracer {
  // TODO: accept config: playfield image
  pub fn new() -> Self {
    Self::at_addr(DEFAULT_ADDR.into())
  }

  pub fn at_addr(addr: SocketAddr) -> Self {
    let (tx, mut rx) = mpsc::unbounded_channel::<TraceEvent>();
    let hub = ConsoleHub::new();

    // ConsoleHub is mainly just a handle to the Arc of Hub state
    // Create two copies, one to receive incoming events from Frontbox
    let recv_hub = hub.clone();
    tokio::spawn(async move {
      while let Some(event) = rx.recv().await {
        recv_hub.ingest(event);
      }
      log::info!(target: "frontbox_pin_console", "Trace event channel closed");
    });
    // And a second to handle web interactions
    tokio::spawn(server::serve(hub.clone(), addr));

    Self { tx, hub }
  }
}

impl AppTracer for WebTracer {
  fn init(&mut self, hardware: &Hardware) {
    self.hub.set_hardware(hardware.clone());
  }

  fn sender(&self) -> mpsc::UnboundedSender<TraceEvent> {
    self.tx.clone()
  }
}
