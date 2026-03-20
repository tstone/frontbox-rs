use std::thread::sleep;
use std::time::Duration;

use frontbox::animation::*;
use frontbox_pin2dmd::*;

fn main() -> rusb::Result<()> {
  let mut dmd = Pin2Dmd::connect(128, 32, PanelType::Rgb)?;

  // animate x position
  let mut x_anim = Tween::new(
    Duration::from_secs(2),
    Curve::BounceOut,
    vec![-30, 100],
    AnimationCycle::Once,
  );

  let frame_rate = 30.0;
  let frame_count = (x_anim.duration.as_secs_f32() * frame_rate).round() as usize;
  let forestman = Asset::from_path(local_asset("forestman.png"));

  for _ in 0..frame_count {
    let tick = Duration::from_millis(33);
    x_anim.tick(tick);

    let mut frame = Frame::for_dmd(&dmd);

    // This clone here is cheap because the asset image is reference-counted and shared across clones,
    // Arc reference and offset coordinates are duplicated
    frame.add(forestman.clone().offset_x(x_anim.sample()));
    dmd.render(&mut frame)?;

    sleep(tick);
  }

  sleep(Duration::from_secs(2));

  dmd.clear()?;
  Ok(())
}

fn local_asset(path: &str) -> String {
  format!("{}/examples/assets/{}", env!("CARGO_MANIFEST_DIR"), path)
}
