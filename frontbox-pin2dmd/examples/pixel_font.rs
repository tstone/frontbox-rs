use std::thread::sleep;
use std::time::Duration;

use fast_protocol::Color;
use frontbox_pin2dmd::*;

fn main() -> rusb::Result<()> {
  let mut dmd = Pin2Dmd::connect(128, 32, PanelType::Rgb)?;
  let mut frame = Frame::new(128, 32, 1);

  let bold_10px = PixelFontBuilder::new()
    .path(concat!(
      env!("CARGO_MANIFEST_DIR"),
      "/examples/assets/bold_pixels.png"
    ))
    .sheet_layout(8, 16)
    .custom_char_width(',', 3)
    .build();

  let mut score1 = 24_990;
  let mut score2 = 19_550;
  let mut score3 = 21_110;
  let mut score4 = 30_000;

  for _ in 0..150 {
    score1 += 1630;
    score2 += 1210;
    score3 += 900;
    score4 += 1590;

    frame.layers[0].overlay_image(
      &bold_10px.render_text(PixelFont::format_number(score1).as_str()),
      2,
      1,
    );

    frame.layers[0].overlay_image(
      &bold_10px.render_text_recolor(PixelFont::format_number(score2).as_str(), Color::coral()),
      2,
      15,
    );

    frame.layers[0].overlay_image(
      &bold_10px.render_text_vgradient(
        PixelFont::format_number(score3).as_str(),
        Color::medium_turquoise(),
        Color::dark_blue(),
      ),
      64,
      1,
    );

    frame.layers[0].overlay_image(
      &bold_10px.render_text_hgradient(
        PixelFont::format_number(score4).as_str(),
        Color::yellow(),
        Color::sea_green(),
      ),
      64,
      15,
    );

    let pixels = frame.to_pixels();
    dmd.render_rgb24(&pixels)?;

    sleep(Duration::from_millis(100));
  }

  dmd.clear()?;
  Ok(())
}
