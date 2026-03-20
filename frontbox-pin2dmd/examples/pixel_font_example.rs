use std::thread::sleep;
use std::time::Duration;

use frontbox_pin2dmd::*;
use image::Rgba;

fn main() -> rusb::Result<()> {
  let mut dmd = Pin2Dmd::connect(128, 32, PanelType::Rgb)?;
  let bold_10px = PixelFontBuilder::new()
    .path(concat!(
      env!("CARGO_MANIFEST_DIR"),
      "/examples/assets/bold_pixels.png"
    ))
    .sheet_layout(8, 16)
    .custom_char_width(',', 3)
    .build();

  let mut frame = dmd.empty_frame();

  let mut score1 = 24_990;
  let mut score2 = 19_550;
  let mut score3 = 21_110;
  let mut score4 = 30_000;

  let left = 5;
  let top = 4;

  for _ in 0..125 {
    score1 += 1630;
    score2 += 1210;
    score3 += 900;
    score4 += 1590;

    frame.add(
      bold_10px
        .text(TextFormatting::number(score1))
        .offset(left, top),
    );
    frame.add(
      bold_10px
        .text(TextFormatting::number(score2))
        .recolor(Rgba::coral())
        .offset(64, top),
    );
    frame.add(
      bold_10px
        .text(TextFormatting::number(score3))
        .recolor_vgradient(Rgba::medium_turquoise(), Rgba::dark_blue())
        .offset(left, 17),
    );
    frame.add(
      bold_10px
        .text(TextFormatting::number(score4))
        .recolor_hgradient(Rgba::yellow(), Rgba::sea_green())
        .offset(64, 17),
    );

    let pixels = frame.to_pixels();
    dmd.render(&pixels)?;

    sleep(Duration::from_millis(100));
  }

  dmd.clear()?;
  Ok(())
}
