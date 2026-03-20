use std::thread::sleep;
use std::time::Duration;

use frontbox_pin2dmd::*;

fn main() -> rusb::Result<()> {
  let mut dmd = Pin2Dmd::connect(128, 32, PanelType::Rgb)?;

  let mut frame = dmd.empty_frame();
  frame.add(Sprite::path(local_asset("bg.png")));
  frame.add(Sprite::path(local_asset("tree1.png")).offset_x(2));
  frame.add(Sprite::path(local_asset("tree2.png")).offset_x(90));
  frame.add(Sprite::path(local_asset("forestman.png")).offset_x(40));
  dmd.render_frame(&frame)?;

  sleep(Duration::from_secs(10));
  dmd.clear()?;
  Ok(())
}

fn local_asset(path: &str) -> String {
  format!("{}/examples/assets/{}", env!("CARGO_MANIFEST_DIR"), path)
}
