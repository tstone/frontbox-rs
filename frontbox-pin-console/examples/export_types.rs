use frontbox::prelude::Hardware;
use frontbox::prelude::app_tracer::TraceEvent;
use ts_rs::{Config, TS};

fn main() {
  let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("web/src/types/generated");
  let cfg = Config::new().with_out_dir(out);

  TraceEvent::export_all(&cfg).unwrap();
  Hardware::export_all(&cfg).unwrap();
}
