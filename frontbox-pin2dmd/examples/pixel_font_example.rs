use std::thread::sleep;
use std::time::Duration;

use frontbox::prelude::*;
use frontbox_pin2dmd::*;

fn main() -> rusb::Result<()> {
  let mut dmd = Pin2Dmd::connect(128, 32, PanelType::Rgb)?;
  let bold_10px = PixelFontBuilder::new()
    .path(
      concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/assets/bold_pixels.png"
      )
      .to_string(),
    )
    .sheet_layout(4, 16)
    .default_char_width(9)
    .custom_char_width(',', 3)
    .build();

  let mut score1 = 24_990;
  let mut score2 = 19_550;
  let mut score3 = 21_110;
  let mut score4 = 30_000;

  let left = 4;
  let top = 4;

  for _ in 0..125 {
    let mut frame = Frame::for_dmd(&dmd);

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
        .recolor(Rgba::pink())
        .offset(66, top),
    );
    frame.add(
      bold_10px
        .text(TextFormatting::number(score3))
        .recolor_vgradient(Rgba::turquoise(), Rgba::blue())
        .offset(left, 17),
    );
    frame.add(
      bold_10px
        .text(TextFormatting::number(score4))
        .recolor_hgradient(Rgba::yellow(), Rgba::green())
        .offset(66, 17),
    );

    dmd.render(&mut frame)?;
    sleep(Duration::from_millis(100));
  }

  dmd.clear()?;
  Ok(())
}
