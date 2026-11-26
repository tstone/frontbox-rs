use frontbox_gfx::scene::Font;

/// The pixel fonts this crate comes with, by the name a scene uses for each, which is where a game's own file would
/// replace it. Who made them is in `assets/fonts/CREDITS.md`.
const FONTS: [(&str, &[u8]); 13] = [
  (
    "frontbox/fonts/DMDSmall.ttf",
    include_bytes!("../assets/fonts/DMDSmall.ttf"),
  ),
  (
    "frontbox/fonts/ImpactBits.ttf",
    include_bytes!("../assets/fonts/ImpactBits.ttf"),
  ),
  (
    "frontbox/fonts/LCDBlock.ttf",
    include_bytes!("../assets/fonts/LCDBlock.ttf"),
  ),
  (
    "frontbox/fonts/Minimal4.ttf",
    include_bytes!("../assets/fonts/Minimal4.ttf"),
  ),
  (
    "frontbox/fonts/MooseNooks.ttf",
    include_bytes!("../assets/fonts/MooseNooks.ttf"),
  ),
  (
    "frontbox/fonts/OwlsNInk.ttf",
    include_bytes!("../assets/fonts/OwlsNInk.ttf"),
  ),
  (
    "frontbox/fonts/SirClive.ttf",
    include_bytes!("../assets/fonts/SirClive.ttf"),
  ),
  (
    "frontbox/fonts/TenFatGuys.ttf",
    include_bytes!("../assets/fonts/TenFatGuys.ttf"),
  ),
  (
    "frontbox/fonts/TimesNewPixel.ttf",
    include_bytes!("../assets/fonts/TimesNewPixel.ttf"),
  ),
  (
    "frontbox/fonts/TinyPix.ttf",
    include_bytes!("../assets/fonts/TinyPix.ttf"),
  ),
  (
    "frontbox/fonts/TinyPixie2.ttf",
    include_bytes!("../assets/fonts/TinyPixie2.ttf"),
  ),
  (
    "frontbox/fonts/VSansSerif8bold.ttf",
    include_bytes!("../assets/fonts/VSansSerif8bold.ttf"),
  ),
  (
    "frontbox/fonts/Webslinger.ttf",
    include_bytes!("../assets/fonts/Webslinger.ttf"),
  ),
];

/// The pixel fonts this crate comes with, by the name a scene uses for them. Each is crisp at its
/// [`Font::pixel_size`], or a whole multiple of it: 16 for most, and 6 for `TinyPixie2`.
pub(crate) fn fonts() -> impl Iterator<Item = (&'static str, Font)> {
  FONTS.into_iter().map(|(name, bytes)| {
    (
      name,
      Font::new(bytes.to_vec()).expect("a bundled font is a font"),
    )
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use frontbox_core::prelude::Vec2;
  use frontbox_gfx::scene::{
    Assets, Color, Fill, FillNode, Layer, LayerFrame, Sample, Shape, ShapeNode, Surface, Transform,
    Variables, no_modulators, no_regions, no_values, no_variables,
  };
  use image::Rgba;

  #[test]
  fn every_bundled_font_loads() {
    assert_eq!(fonts().count(), FONTS.len());
  }

  #[test]
  fn every_bundled_font_is_on_whole_units_at_its_design_size() {
    let (width, height) = (240, 24);
    for (name, font) in fonts() {
      let size = font
        .pixel_size()
        .unwrap_or_else(|| panic!("{name} is a pixel font"));
      let expected = if name == "frontbox/fonts/TinyPixie2.ttf" {
        6.0
      } else {
        16.0
      };
      assert_eq!(size, expected, "{name}");
      let mut assets = Assets::new();
      assets.insert_font(name, font);
      let text = Shape::new(ShapeNode::text(
        "Pinball 1,250 EXTRA!",
        name,
        size,
        Vec2::new(width as f32, height as f32),
      ))
      .transform(Transform::Translate(
        (Vec2::new(width as f32, height as f32) / 2.0).into(),
      ));
      let layer = Layer::new(Fill::new(FillNode::Solid(
        Color(Rgba([255, 255, 255, 255])).into(),
      )))
      .region(text);
      let variables = Variables::new();
      let size = Vec2::new(width as f32, height as f32);
      let frame = LayerFrame {
        time: 0.0,
        playback_seed: 0,
        variables: &variables,
        defaults: no_variables(),
        values: no_values(),
        modulators: no_modulators(),
        regions: no_regions(),
        assets: &assets,
        size,
        surface: Surface::Area,
        visits: None,
        timing: None,
      };
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
