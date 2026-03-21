use rand::RngExt;
use rand::prelude::IteratorRandom;
use std::thread::sleep;
use std::time::Duration; // Import the Rng trait

use frontbox::time::*;
use frontbox_pin2dmd::*;
use image::Rgba;

fn main() -> rusb::Result<()> {
  let mut dmd = Pin2Dmd::connect(128, 32, PanelType::Rgb)?;

  // animate x position
  let mut x_anim = Tween::new(
    Duration::from_secs(2),
    Curve::EaseInOut,
    vec![130, -1],
    AnimationCycle::Forever,
  );

  let mut frame_anim = Tween::new(
    Duration::from_millis(100),
    Curve::Linear,
    vec![0, 2],
    AnimationCycle::Forever,
  );

  let mut color_anim = Tween::new(
    Duration::from_secs(2),
    Curve::Linear,
    vec![Rgba::orange_red(), Rgba::rebecca_purple()],
    AnimationCycle::Forever,
  );

  let mut facing_left = true;
  let bird_sheet = SpriteSheet::new(local_asset("flying.png"), 1, 3);

  let bold_10px = PixelFontBuilder::new()
    .path(local_asset("bold_pixels.png"))
    .sheet_layout(4, 16)
    .default_char_width(9)
    .custom_char_width(',', 3)
    .build();

  let mut rng = rand::rng();
  let mut score = 10_000;

  loop {
    let tick = Duration::from_millis(33);
    x_anim.tick(tick);
    color_anim.tick(tick);
    frame_anim.tick(tick);

    let mut frame =
      Frame::for_dmd(&dmd).with_fill(Fill::VerticalGradient(Rgba::black(), Rgba::dark_blue()));

    // This clone here is cheap because the asset image is reference-counted and shared across clones,
    // Arc reference and offset coordinates are duplicated
    if facing_left {
      frame.add(
        bird_sheet
          .image_at(0, frame_anim.sample())
          .left(x_anim.sample()),
      );
    } else {
      frame.add(
        bird_sheet
          .image_at(0, frame_anim.sample())
          .fliph()
          .left(x_anim.sample()),
      );
    }

    if rng.random_bool(0.4) {
      score += (10..40000).choose(&mut rng).unwrap_or(150);
    }

    frame.add(
      bold_10px
        .text(TextFormatting::number(score))
        .recolor_vgradient(color_anim.sample(), Rgba::antique_white())
        .bottom(1)
        .right(4),
    );

    dmd.render(&mut frame)?;

    sleep(tick);

    if rng.random_bool(0.015) {
      let current_x = x_anim.sample();
      facing_left = rng.random_bool(0.5);

      let random_next_x = if facing_left {
        (0..current_x / 2).choose(&mut rng).unwrap_or(0)
      } else {
        ((current_x + 1)..(128) - 32)
          .choose(&mut rng)
          .unwrap_or(128 - 32)
      };

      let millis = (500..2000).choose(&mut rng).unwrap_or(1000);
      x_anim = Tween::new(
        Duration::from_millis(millis),
        Curve::EaseInOut,
        vec![current_x, random_next_x],
        AnimationCycle::Forever,
      );
    }
  }
}

fn local_asset(path: &str) -> String {
  format!("{}/examples/assets/{}", env!("CARGO_MANIFEST_DIR"), path)
}
