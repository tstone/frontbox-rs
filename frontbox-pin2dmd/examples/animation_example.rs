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
    AnimationCycle::Forever,
  );

  let frame_rate = 30.0; // TODO: make this configurable?
  let frame_count = (x_anim.duration.as_secs_f32() * frame_rate).round() as usize;

  for _ in 0..frame_count {
    let tick = Duration::from_millis(33);
    x_anim.tick(tick);

    let mut frame = dmd.empty_frame();

    frame.add(Sprite::path(local_asset("forestman.png")).offset_x(x_anim.sample()));

    dmd.render(&mut frame, tick)?;
    sleep(tick);
  }

  sleep(Duration::from_secs(2));

  dmd.clear()?;
  Ok(())
}

fn local_asset(path: &str) -> String {
  format!("{}/examples/assets/{}", env!("CARGO_MANIFEST_DIR"), path)
}
