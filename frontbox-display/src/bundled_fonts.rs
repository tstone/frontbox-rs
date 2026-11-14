use frontbox_gfx::scene::Font;

/// The pixel fonts this crate comes with, by the name a scene uses for each. Who made them is in
/// `assets/fonts/CREDITS.md`.
const FONTS: [(&str, &[u8]); 12] = [
  ("DMDSmall", include_bytes!("../assets/fonts/DMDSmall.ttf")),
  ("ImpactBits", include_bytes!("../assets/fonts/ImpactBits.ttf")),
  ("LCDBlock", include_bytes!("../assets/fonts/LCDBlock.ttf")),
  ("Minimal4", include_bytes!("../assets/fonts/Minimal4.ttf")),
  ("MooseNooks", include_bytes!("../assets/fonts/MooseNooks.ttf")),
  ("OwlsNInk", include_bytes!("../assets/fonts/OwlsNInk.ttf")),
  ("SirClive", include_bytes!("../assets/fonts/SirClive.ttf")),
  ("TimesNewPixel", include_bytes!("../assets/fonts/TimesNewPixel.ttf")),
  ("TinyPix", include_bytes!("../assets/fonts/TinyPix.ttf")),
  ("TinyPixie2", include_bytes!("../assets/fonts/TinyPixie2.ttf")),
  ("TomAndJerry", include_bytes!("../assets/fonts/TomAndJerry.ttf")),
  ("VSansSerif8bold", include_bytes!("../assets/fonts/VSansSerif8bold.ttf")),
];

/// The pixel fonts this crate comes with, by the name a scene uses for them, to give to
/// [`Content::with_fonts`](frontbox_gfx::scene::Content::with_fonts). Each is crisp at its
/// [`Font::pixel_size`], or a whole multiple of it: 16 for most, and 6 for `TinyPixie2`.
pub fn bundled_fonts() -> impl Iterator<Item = (&'static str, Font)> {
  FONTS.into_iter().map(|(name, bytes)| (name, Font::new(bytes.to_vec()).expect("a bundled font is a font")))
}

#[cfg(test)]
mod tests {
  use super::*;
  use frontbox_core::prelude::Vec2;
  use frontbox_gfx::scene::{Assets, Color, Fill, FillNode, Layer, LayerFrame, Params, Sample, Shape, ShapeNode, Surface, Transform, no_params};
  use image::Rgba;

  #[test]
  fn every_bundled_font_loads() {
    assert_eq!(bundled_fonts().count(), FONTS.len());
  }

  #[test]
  fn every_bundled_font_is_on_whole_units_at_its_design_size() {
    let (width, height) = (240, 24);
    for (name, font) in bundled_fonts() {
      let size = font.pixel_size().unwrap_or_else(|| panic!("{name} is a pixel font"));
      let expected = if name == "TinyPixie2" { 6.0 } else { 16.0 };
      assert_eq!(size, expected, "{name}");
      let mut assets = Assets::new();
      assets.insert_font(name, font);
      let text = Shape::new(ShapeNode::text("Pinball 1,250 EXTRA!", name, size, Vec2::new(width as f32, height as f32)))
        .transform(Transform::Translate((Vec2::new(width as f32, height as f32) / 2.0).into()));
      let layer = Layer::new(Fill::new(FillNode::Solid(Color(Rgba([255, 255, 255, 255])).into()))).region(text);
      let params = Params::new();
      let size = Vec2::new(width as f32, height as f32);
      let frame = LayerFrame { time: 0.0, playback_seed: 0, params: &params, defaults: no_params(), assets: &assets, size, surface: Surface::Area, visits: None, timing: None };
      // a sample half a unit across is only partly covered where an edge falls between whole units. A font made on a
      // grid that doesn't divide its em evenly, such as 6 pixels to 1024 units, is a rounding error off, which adds up
      // along a line to a few hundredths of a unit here.
      let layer = layer.prepare(&frame, 0.5);
      let alphas: Vec<u8> = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| layer.sample(&Sample::new(Vec2::new(x as f32 + 0.5, y as f32 + 0.5), 0.5))[3])
        .collect();
      assert!(alphas.contains(&255), "{name} draws something");
      let off_grid = alphas.iter().find(|alpha| (11..=244).contains(*alpha));
      assert_eq!(off_grid, None, "{name} has edges between whole units");
    }
  }
}
