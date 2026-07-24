use frontbox::prelude::color_sequence::GradientStop;
use frontbox::prelude::*;
use frontbox_canvas::*;

pub struct DmdMenuTheme {
  pub menu_bg: Fill2d,
  pub menu_border: Option<Border>,
  pub unselected_section_color: Rgba<u8>,
  pub unselected_section_bg: Fill2d,
  pub selected_section_color: Rgba<u8>,
  pub selected_section_bg: Fill2d,
  pub unselected_config_color: Rgba<u8>,
  pub unselected_config_bg: Fill2d,
  pub selected_config_color: Rgba<u8>,
  pub selected_config_bg: Fill2d,
}

impl Default for DmdMenuTheme {
  fn default() -> Self {
    let unselected_bg = Fill2d::Transparent;
    let unselected_color = Rgba::yellow().darken(0.15);
    let selected_bg = Fill2d::Gradient(
      vec![
        GradientStop::new(0.0, Rgba::cyan().lighten(0.15)),
        GradientStop::new(1.0, Rgba::white()),
      ],
      45.0,
    );
    let selected_color = Rgba::black();

    DmdMenuTheme {
      menu_bg: Fill2d::Transparent,
      menu_border: Some(Border::new(1, Rgba::blue().darken(0.15))),
      // unselected
      unselected_section_bg: unselected_bg.clone(),
      unselected_config_bg: unselected_bg,
      unselected_section_color: unselected_color,
      unselected_config_color: unselected_color,
      // selected
      selected_config_bg: selected_bg.clone(),
      selected_section_bg: selected_bg,
      selected_config_color: selected_color,
      selected_section_color: selected_color,
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

    let unselected_bg = Fill2d::Transparent;
    let unselected_color = sky;

    // Sections get a cool "ocean" gradient when selected
    let selected_section_bg = Fill2d::Gradient(
      vec![GradientStop::new(0.0, ocean), GradientStop::new(1.0, sky)],
      45.0,
    );

    // Configs get a warm "sunset" gradient when selected, to visually
    // distinguish the two row kinds using the same palette
    let selected_config_bg = Fill2d::Gradient(
      vec![GradientStop::new(0.0, sunset), GradientStop::new(1.0, sun)],
      45.0,
    );

    DmdMenuTheme {
      menu_bg: Fill2d::Transparent,
      menu_border: Some(Border::new(1, deep)),
      // unselected
      unselected_section_bg: unselected_bg.clone(),
      unselected_config_bg: unselected_bg,
      unselected_section_color: unselected_color,
      unselected_config_color: unselected_color,
      // selected
      selected_section_bg,
      selected_config_bg,
      selected_section_color: deep,
      selected_config_color: deep,
    }
  }

  /// Dark, high-contrast theme inspired by a popular programming editor color schemes
  pub fn carbon() -> Self {
    let bg = Rgba([0x27, 0x28, 0x22, 0xff]); // near-black charcoal
    let fg = Rgba([0xf8, 0xf8, 0xf2, 0xff]); // off-white
    let pink = Rgba([0xf9, 0x26, 0x72, 0xff]);
    let green = Rgba([0xa6, 0xe2, 0x2e, 0xff]);
    let orange = Rgba([0xfd, 0x97, 0x1f, 0xff]);
    let blue = Rgba([0x66, 0xd9, 0xef, 0xff]);

    let unselected_bg = Fill2d::Transparent;
    let unselected_color = fg;

    // Sections selected: cool blue/green sweep
    let selected_section_bg = Fill2d::Gradient(
      vec![GradientStop::new(0.0, blue), GradientStop::new(1.0, green)],
      45.0,
    );

    // Configs selected: warm pink/orange sweep, mirroring the section/config
    // split from the summer theme
    let selected_config_bg = Fill2d::Gradient(
      vec![GradientStop::new(0.0, pink), GradientStop::new(1.0, orange)],
      45.0,
    );

    DmdMenuTheme {
      menu_bg: Fill2d::Solid(bg),
      menu_border: Some(Border::new(1, fg.darken(0.3))),
      // unselected
      unselected_section_bg: unselected_bg.clone(),
      unselected_config_bg: unselected_bg,
      unselected_section_color: unselected_color,
      unselected_config_color: unselected_color,
      // selected
      selected_section_bg,
      selected_config_bg,
      selected_section_color: bg,
      selected_config_color: bg,
    }
  }

  /// The deep purples of sundown are the herald of darkness
  pub fn twilight() -> Self {
    let dusk = Rgba([0x2b, 0x1a, 0x4a, 0xff]); // deep purple, near-black
    let violet = Rgba([0x6a, 0x4c, 0x93, 0xff]); // muted violet
    let periwinkle = Rgba([0x8a, 0x7c, 0xd8, 0xff]); // soft blue-violet
    let rose = Rgba([0xe0, 0x7a, 0x8f, 0xff]); // fading sunset pink
    let ember = Rgba([0xf2, 0xa6, 0x5a, 0xff]); // last light of the horizon

    let unselected_bg = Fill2d::Transparent;
    let unselected_color = periwinkle;

    // Sections selected: cool violet/periwinkle sweep, evoking deepening sky
    let selected_section_bg = Fill2d::Gradient(
      vec![
        GradientStop::new(0.0, violet),
        GradientStop::new(1.0, periwinkle),
      ],
      45.0,
    );

    // Configs selected: warm rose/ember sweep, the fading horizon glow
    let selected_config_bg = Fill2d::Gradient(
      vec![GradientStop::new(0.0, rose), GradientStop::new(1.0, ember)],
      45.0,
    );

    DmdMenuTheme {
      menu_bg: Fill2d::Solid(dusk),
      menu_border: Some(Border::new(1, violet.lighten(0.1))),
      // unselected
      unselected_section_bg: unselected_bg.clone(),
      unselected_config_bg: unselected_bg,
      unselected_section_color: unselected_color,
      unselected_config_color: unselected_color,
      // selected
      selected_section_bg,
      selected_config_bg,
      selected_section_color: dusk,
      selected_config_color: dusk,
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

    let unselected_bg = Fill2d::Transparent;
    let unselected_color = peach;

    // Sections selected: flat, punchy solid block — no blend
    let selected_section_bg = Fill2d::Solid(coral);

    // Configs selected: hard two-tone gradient (sharp color relationship,
    // not a soft fade) to keep the "graphic," not "atmospheric," feel
    let selected_config_bg = Fill2d::Gradient(
      vec![
        GradientStop::new(0.0, navy),
        GradientStop::new(0.5, navy),
        GradientStop::new(0.5, plum),
        GradientStop::new(1.0, plum),
      ],
      0.0,
    );

    DmdMenuTheme {
      menu_bg: Fill2d::Solid(charcoal),
      menu_border: Some(Border::new(1, peach)),
      // unselected
      unselected_section_bg: unselected_bg.clone(),
      unselected_config_bg: unselected_bg,
      unselected_section_color: unselected_color,
      unselected_config_color: unselected_color,
      // selected
      selected_section_bg,
      selected_config_bg,
      selected_section_color: charcoal,
      selected_config_color: rose,
    }
  }
}
