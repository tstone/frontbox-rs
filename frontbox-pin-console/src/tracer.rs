use frontbox::prelude::{Hardware, app_tracer::*};
use tokio::sync::mpsc;

pub struct WebTracer {
  tx: mpsc::UnboundedSender<TraceEvent>,
}

impl WebTracer {
  // TODO: accept config: playfield image
  pub fn new() -> Self {
    let (tx, _rx) = mpsc::unbounded_channel::<TraceEvent>();
    tokio::spawn(async move { /* WebInterface::new().run(rx).await */ });
    Self { tx }
  }
}

impl AppTracer for WebTracer {
  fn init(&mut self, _hardware: &Hardware) {
    // TODO pass this to the web interface
  }

  fn sender(&self) -> mpsc::UnboundedSender<TraceEvent> {
    self.tx.clone()
  }
}
