use frontbox::prelude::Hardware;
use frontbox::prelude::app_tracer::TraceEvent;
use frontbox_pin_console::protocol::ServerMessage;
use ts_rs::{Config, TS};

fn main() {
  let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("web/src/types/generated");
  // JSON has no 64-bit integers; serde_json emits u64 as a plain number
  let cfg = Config::new().with_out_dir(out).with_large_int("number");

  TraceEvent::export_all(&cfg).unwrap();
  Hardware::export_all(&cfg).unwrap();
  ServerMessage::export_all(&cfg).unwrap();
}
