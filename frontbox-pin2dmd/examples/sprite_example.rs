use std::thread::sleep;
use std::time::Duration;

use frontbox_pin2dmd::*;

fn main() -> rusb::Result<()> {
  let mut dmd = Pin2Dmd::connect(128, 32, PanelType::Rgb)?;

  let mut frame = Frame::for_dmd(&dmd);

  frame.add(Asset::from_path(local_asset("bg.png")));
  frame.add(Asset::from_path(local_asset("tree1.png")).left(2));
  frame.add(Asset::from_path(local_asset("tree2.png")).left(90));
  frame.add(Asset::from_path(local_asset("forestman.png")).left(40));

  dmd.render(&mut frame)?;
  sleep(Duration::from_secs(8));
  dmd.clear()?;

  Ok(())
}

fn local_asset(path: &str) -> String {
  format!("{}/examples/assets/{}", env!("CARGO_MANIFEST_DIR"), path)
}
