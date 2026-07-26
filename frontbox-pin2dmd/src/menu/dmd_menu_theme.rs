use frontbox::prelude::color_sequence::GradientStop;
use frontbox::prelude::*;
use frontbox_canvas::*;

pub struct DmdMenuTheme {
  pub menu_bg: Fill2d,
  pub menu_border: Option<Border>,
  pub heading_color: Rgba<u8>,
  pub heading_bg: Fill2d,
  pub selected_color: Rgba<u8>,
  pub selected_bg: Fill2d,
  pub unselected_color: Rgba<u8>,
  pub unselected_bg: Fill2d,
}

impl Default for DmdMenuTheme {
  fn default() -> Self {
    DmdMenuTheme {
      menu_bg: Fill2d::Transparent,
      menu_border: Some(Border::new(1, Rgba::blue().darken(0.15))),
      heading_bg: Fill2d::Solid(Rgba([0x27, 0x28, 0x22, 0xff])),
      heading_color: Rgba::black().lighten(0.85),
      unselected_bg: Fill2d::Transparent,
      unselected_color: Rgba::yellow().lighten(0.3),
      selected_bg: Fill2d::Transparent,
      selected_color: Rgba::cyan().lighten(0.3),
    }
  }
}

impl DmdMenuTheme {
  /// Ice cream, board shorts, and the sound of the ocean
  pub fn summer() -> Self {
    let sky = Rgba([0x8e, 0xca, 0xe6, 0xff]); // #8ecae6
    let ocean = Rgba([0x21, 0x9e, 0xbc, 0xff]); // #219ebc
    let deep = Rgba([0x02, 0x30, 0x47, 0xff]); // #023047
    let sun = Rgba([0xff, 0xb7, 0x03, 0xff]); // #ffb703
    let sunset = Rgba([0xfb, 0x85, 0x00, 0xff]); // #fb8500

    DmdMenuTheme {
      menu_bg: Fill2d::Transparent,
      menu_border: Some(Border::new(1, sun)),
      heading_bg: Fill2d::Transparent,
      heading_color: sunset,
      unselected_bg: Fill2d::Transparent,
      unselected_color: sunset,
      selected_bg: Fill2d::Gradient(
        vec![GradientStop::new(0.0, ocean), GradientStop::new(1.0, sky)],
        45.0,
      ),
      selected_color: deep,
    }
  }

  /// Dark, high-contrast theme inspired by a popular programming editor color schemes
  pub fn sherbet() -> Self {
    let bg = Rgba([0x27, 0x28, 0x22, 0xff]); // near-black charcoal
    let fg = Rgba([0xf8, 0xf8, 0xf2, 0xff]); // off-white
    let pink = Rgba([0xf9, 0x26, 0x72, 0xff]);
    let green = Rgba([0xa6, 0xe2, 0x2e, 0xff]);
    // let orange = Rgba([0xfd, 0x97, 0x1f, 0xff]);
    let blue = Rgba([0x66, 0xd9, 0xef, 0xff]).darken(0.25);

    DmdMenuTheme {
      menu_bg: Fill2d::Solid(bg),
      menu_border: Some(Border::new(1, fg.darken(0.3))),
      heading_bg: Fill2d::Transparent,
      heading_color: green,
      unselected_bg: Fill2d::Transparent,
      unselected_color: fg.darken(0.25),
      selected_bg: Fill2d::Gradient(
        vec![
          GradientStop::new(0.0, pink.darken(0.1)),
          GradientStop::new(0.6, blue),
          GradientStop::new(1.0, fg.darken(0.4)),
        ],
        100.5,
      ),
      selected_color: Rgba::black(),
    }
  }

  /// The deep purples of sundown are the herald of darkness
  pub fn twilight() -> Self {
    let dusk = Rgba([0x2b, 0x1a, 0x4a, 0xff]); // deep purple, near-black
    let violet = Rgba([0x6a, 0x4c, 0x93, 0xff]); // muted violet
    let periwinkle = Rgba([0x8a, 0x7c, 0xd8, 0xff]); // soft blue-violet
    let rose = Rgba([0xe0, 0x7a, 0x8f, 0xff]); // fading sunset pink
    let ember = Rgba([0xf2, 0xa6, 0x5a, 0xff]); // last light of the horizon

    DmdMenuTheme {
      menu_bg: Fill2d::Solid(dusk),
      menu_border: Some(Border::new(1, violet.lighten(0.1))),
      heading_bg: Fill2d::Solid(ember),
      heading_color: rose,
      unselected_bg: Fill2d::Transparent,
      unselected_color: periwinkle,
      selected_bg: Fill2d::Gradient(
        vec![
          GradientStop::new(0.0, violet),
          GradientStop::new(1.0, periwinkle),
        ],
        45.0,
      ),
      selected_color: Rgba::black(),
    }
  }

  /// Bold, flat color-block pop art inspired theme
  pub fn pop_art() -> Self {
    let charcoal = Rgba([0x22, 0x22, 0x22, 0xff]); // dark gray/charcoal bg
    let navy = Rgba([0x35, 0x50, 0x70, 0xff]); // #355070
    let plum = Rgba([0x6d, 0x59, 0x7a, 0xff]); // #6d597a
    let rose = Rgba([0xb5, 0x65, 0x76, 0xff]); // #b56576
    let coral = Rgba([0xe5, 0x6b, 0x6f, 0xff]); // #e56b6f
    let peach = Rgba([0xea, 0xac, 0x8b, 0xff]); // #eaac8b

    DmdMenuTheme {
      menu_bg: Fill2d::Solid(charcoal),
      menu_border: Some(Border::new(1, peach)),
      heading_bg: Fill2d::Solid(navy),
      heading_color: Rgba::cyan(),
      unselected_bg: Fill2d::Transparent,
      unselected_color: rose,
      selected_bg: Fill2d::Gradient(
        vec![GradientStop::new(0.0, coral), GradientStop::new(1.0, plum)],
        95.0,
      ),
      selected_color: Rgba::black(),
    }
  }
}
