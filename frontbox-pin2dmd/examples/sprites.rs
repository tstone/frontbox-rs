use std::thread::sleep;
use std::time::Duration;

use frontbox_pin2dmd::*;

fn main() -> rusb::Result<()> {
  let mut dmd = Pin2Dmd::connect(128, 32, PanelType::Rgb)?;
  let mut frame = Frame::new(128, 32, 2);
  frame.layers[0].overlay_path(
    concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/bg.png"),
    0,
    0,
  );
  frame.layers[1].overlay_path(
    concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/forestman.png"),
    40,
    0,
  );
  frame.layers[1].overlay_path(
    concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/tree1.png"),
    0,
    0,
  );
  frame.layers[1].overlay_path(
    concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/tree2.png"),
    90,
    0,
  );

  let pixels = frame.to_pixels();
  dmd.render_rgb24(&pixels)?;

  sleep(Duration::from_secs(10));
  dmd.clear()?;
  Ok(())
}
