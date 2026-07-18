use fast_protocol::Color;
use image::Rgba;
use palette::{FromColor, Hsl, IntoColor, Srgb};

use crate::prelude::LedChannels;

pub trait RgbaColor {
  fn red() -> Self;
  fn green() -> Self;
  fn blue() -> Self;
  fn white() -> Self;
  fn black() -> Self;
  fn yellow() -> Self;
  fn pink() -> Self;
  fn light_pink() -> Self;
  fn hot_pink() -> Self;
  fn deep_pink() -> Self;
  fn pale_violet_red() -> Self;
  fn light_salmon() -> Self;
  fn salmon() -> Self;
  fn dark_salmon() -> Self;
  fn light_coral() -> Self;
  fn crimson() -> Self;
  fn firebrick() -> Self;
  fn dark_red() -> Self;
  fn maroon() -> Self;
  fn orange_red() -> Self;
  fn tomato() -> Self;
  fn coral() -> Self;
  fn dark_orange() -> Self;
  fn orange() -> Self;
  fn light_yellow() -> Self;
  fn lemon_chiffon() -> Self;
  fn light_goldenrod_yellow() -> Self;
  fn papaya_whip() -> Self;
  fn moccasin() -> Self;
  fn peach_puff() -> Self;
  fn pale_goldenrod() -> Self;
  fn khaki() -> Self;
  fn dark_khaki() -> Self;
  fn gold() -> Self;
  fn cornsilk() -> Self;
  fn blanched_almond() -> Self;
  fn bisque() -> Self;
  fn navajo_white() -> Self;
  fn wheat() -> Self;
  fn burly_wood() -> Self;
  fn tan() -> Self;
  fn rosy_brown() -> Self;
  fn sandy_brown() -> Self;
  fn goldenrod() -> Self;
  fn dark_goldenrod() -> Self;
  fn peru() -> Self;
  fn chocolate() -> Self;
  fn saddle_brown() -> Self;
  fn sienna() -> Self;
  fn brown() -> Self;
  fn dark_olive_green() -> Self;
  fn olive() -> Self;
  fn olive_drab() -> Self;
  fn yellow_green() -> Self;
  fn lime_green() -> Self;
  fn lime() -> Self;
  fn lawn_green() -> Self;
  fn chartreuse() -> Self;
  fn green_yellow() -> Self;
  fn spring_green() -> Self;
  fn medium_spring_green() -> Self;
  fn light_green() -> Self;
  fn pale_green() -> Self;
  fn dark_sea_green() -> Self;
  fn medium_sea_green() -> Self;
  fn sea_green() -> Self;
  fn forest_green() -> Self;
  fn dark_green() -> Self;
  fn medium_aquamarine() -> Self;
  fn aquamarine() -> Self;
  fn light_cyan() -> Self;
  fn cyan() -> Self;
  fn aqua() -> Self;
  fn pale_turquoise() -> Self;
  fn turquoise() -> Self;
  fn medium_turquoise() -> Self;
  fn dark_turquoise() -> Self;
  fn light_sea_green() -> Self;
  fn cadet_blue() -> Self;
  fn dark_cyan() -> Self;
  fn teal() -> Self;
  fn light_steel_blue() -> Self;
  fn powder_blue() -> Self;
  fn light_blue() -> Self;
  fn sky_blue() -> Self;
  fn light_sky_blue() -> Self;
  fn deep_sky_blue() -> Self;
  fn dodger_blue() -> Self;
  fn cornflower_blue() -> Self;
  fn steel_blue() -> Self;
  fn royal_blue() -> Self;
  fn medium_blue() -> Self;
  fn dark_blue() -> Self;
  fn navy() -> Self;
  fn midnight_blue() -> Self;
  fn medium_slate_blue() -> Self;
  fn slate_blue() -> Self;
  fn dark_slate_blue() -> Self;
  fn lavender() -> Self;
  fn thistle() -> Self;
  fn plum() -> Self;
  fn violet() -> Self;
  fn orchid() -> Self;
  fn fuchsia() -> Self;
  fn magenta() -> Self;
  fn medium_orchid() -> Self;
  fn medium_purple() -> Self;
  fn blue_violet() -> Self;
  fn dark_violet() -> Self;
  fn dark_orchid() -> Self;
  fn dark_magenta() -> Self;
  fn purple() -> Self;
  fn indigo() -> Self;
  fn dark_slate_gray() -> Self;
  fn white_smoke() -> Self;
  fn honeydew() -> Self;
  fn mint_cream() -> Self;
  fn azure() -> Self;
  fn alice_blue() -> Self;
  fn ghost_white() -> Self;
  fn sea_shell() -> Self;
  fn beige() -> Self;
  fn old_lace() -> Self;
  fn floral_white() -> Self;
  fn ivory() -> Self;
  fn antique_white() -> Self;
  fn linen() -> Self;
  fn lavender_blush() -> Self;
  fn misty_rose() -> Self;
  fn gainsboro() -> Self;
  fn light_gray() -> Self;
  fn silver() -> Self;
  fn dark_gray() -> Self;
  fn gray() -> Self;
  fn dim_gray() -> Self;
  fn light_slate_gray() -> Self;
  fn slate_gray() -> Self;
  fn dark_slate_grey() -> Self;
  fn charcoal() -> Self;
  fn rebecca_purple() -> Self;

  fn off() -> Self;
  fn default() -> Self;

  fn to_color(self) -> Color;
  fn remap(self, channels: LedChannels) -> Self;

  fn mix(a: Self, b: Self, t: f32) -> Self;
  fn mix_with(&self, b: Self, t: f32) -> Self;

  fn with_red(self, red: u8) -> Self;
  fn with_red_f32(self, red: f32) -> Self;
  fn with_blue(self, blue: u8) -> Self;
  fn with_blue_f32(self, blue: f32) -> Self;
  fn with_green(self, green: u8) -> Self;
  fn with_green_f32(self, green: f32) -> Self;
  fn with_alpha(self, alpha: u8) -> Self;
  fn with_alpha_f32(self, alpha: f32) -> Self;
  fn with_hue(self, value: f32) -> Self;
  fn with_saturation(self, value: f32) -> Self;
  fn with_lightness(self, value: f32) -> Self;
  fn with_gamma(self, value: f32) -> Self;
  fn with_hue_shift(self, degrees: f32) -> Self;
  fn inverted(self) -> Self;

  fn over(src: Self, dst: Self) -> Self;
  fn composite_over(&self, dst: Self) -> Self;
}

impl RgbaColor for Rgba<u8> {
  fn to_color(self) -> Color {
    Color::rgb(self[0], self[1], self[2])
  }

  /// Re-map the current color, based on given channel order
  fn remap(self, channels: LedChannels) -> Self {
    match channels {
      LedChannels::RGB | LedChannels::RGBW => self,
      LedChannels::BRG | LedChannels::BRGW => Rgba([self[2], self[0], self[1], 0]),
      LedChannels::GRB | LedChannels::GRBW => Rgba([self[1], self[0], self[2], 0]),
    }
  }

  /// Combine two colors, where `t` is the weight of `b` (0.0 = all `a`, 1.0 = all `b`)
  fn mix(a: Rgba<u8>, b: Rgba<u8>, t: f32) -> Rgba<u8> {
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Rgba([
      lerp(a.0[0], b.0[0]),
      lerp(a.0[1], b.0[1]),
      lerp(a.0[2], b.0[2]),
      lerp(a.0[3], b.0[3]),
    ])
  }

  /// Combine with b, where `t` is the weight of `b` (0.0 = all `a`, 1.0 = all `b`)
  fn mix_with(&self, b: Rgba<u8>, t: f32) -> Rgba<u8> {
    Self::mix(*self, b, t)
  }

  fn with_hue(self, degrees: f32) -> Self {
    let (mut hsl, a) = to_hsl(self);
    hsl.hue = palette::RgbHue::from_degrees(degrees.rem_euclid(360.0));
    from_hsl(hsl, a)
  }

  fn with_saturation(self, value: f32) -> Self {
    let (mut hsl, a) = to_hsl(self);
    hsl.saturation = value.clamp(0.0, 1.0);
    from_hsl(hsl, a)
  }

  fn with_lightness(self, value: f32) -> Self {
    let (mut hsl, a) = to_hsl(self);
    hsl.lightness = value.clamp(0.0, 1.0);
    from_hsl(hsl, a)
  }

  fn with_gamma(self, value: f32) -> Self {
    let mut new = self.clone();
    for c in 0..3 {
      let n = new.0[c] as f32 / 255.0;
      new.0[c] = (n.powf(value) * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    new
  }

  fn with_hue_shift(self, degrees: f32) -> Self {
    let (mut hsl, a) = to_hsl(self);
    hsl.hue += palette::RgbHue::from_degrees(degrees);
    from_hsl(hsl, a)
  }

  fn inverted(self) -> Self {
    let mut new = self.clone();
    for c in 0..3 {
      new.0[c] = 255 - new.0[c];
    }
    new
  }

  fn with_red(self, red: u8) -> Self {
    Rgba([red, self.0[1], self.0[2], self.0[3]])
  }

  fn with_red_f32(self, red: f32) -> Self {
    self.with_red((red * 255.0) as u8)
  }

  fn with_blue(self, blue: u8) -> Self {
    Rgba([self.0[0], self.0[1], blue, self.0[3]])
  }

  fn with_blue_f32(self, blue: f32) -> Self {
    self.with_blue((blue * 255.0) as u8)
  }

  fn with_green(self, green: u8) -> Self {
    Rgba([self.0[0], green, self.0[2], self.0[3]])
  }

  fn with_green_f32(self, green: f32) -> Self {
    self.with_green((green * 255.0) as u8)
  }

  fn with_alpha(self, alpha: u8) -> Self {
    Rgba([self.0[0], self.0[1], self.0[2], alpha])
  }

  fn with_alpha_f32(self, alpha: f32) -> Self {
    self.with_alpha((alpha * 255.0) as u8)
  }

  /// Composite `src` over `dst` using alpha blending, e.g. red over white with 50% alpha results in pink
  fn over(src: Rgba<u8>, dst: Rgba<u8>) -> Rgba<u8> {
    let sa = src.0[3] as f32 / 255.0;
    let da = dst.0[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a == 0.0 {
      return Rgba([0, 0, 0, 0]);
    }
    let blend = |s: u8, d: u8| ((s as f32 * sa + d as f32 * da * (1.0 - sa)) / out_a) as u8;
    Rgba([
      blend(src.0[0], dst.0[0]),
      blend(src.0[1], dst.0[1]),
      blend(src.0[2], dst.0[2]),
      (out_a * 255.0) as u8,
    ])
  }

  /// Composite over top of `dst` using alpha blending, e.g. red over white with 50% alpha results in pink
  fn composite_over(&self, dst: Self) -> Self {
    Self::over(*self, dst)
  }

  fn red() -> Self {
    Rgba([255, 0, 0, 255])
  }
  fn green() -> Self {
    Rgba([0, 255, 0, 255])
  }
  fn blue() -> Self {
    Rgba([0, 0, 255, 255])
  }
  fn white() -> Self {
    Rgba([255, 255, 255, 255])
  }
  fn black() -> Self {
    Rgba([0, 0, 0, 255])
  }
  fn yellow() -> Self {
    Rgba([255, 255, 0, 255])
  }
  fn pink() -> Self {
    Rgba([255, 192, 203, 255])
  }
  fn light_pink() -> Self {
    Rgba([255, 182, 193, 255])
  }
  fn hot_pink() -> Self {
    Rgba([255, 105, 180, 255])
  }
  fn deep_pink() -> Self {
    Rgba([255, 20, 147, 255])
  }
  fn pale_violet_red() -> Self {
    Rgba([219, 112, 147, 255])
  }
  fn light_salmon() -> Self {
    Rgba([255, 160, 122, 255])
  }
  fn salmon() -> Self {
    Rgba([250, 128, 114, 255])
  }
  fn dark_salmon() -> Self {
    Rgba([233, 150, 122, 255])
  }
  fn light_coral() -> Self {
    Rgba([240, 128, 128, 255])
  }
  fn crimson() -> Self {
    Rgba([220, 20, 60, 255])
  }
  fn firebrick() -> Self {
    Rgba([178, 34, 34, 255])
  }
  fn dark_red() -> Self {
    Rgba([139, 0, 0, 255])
  }
  fn maroon() -> Self {
    Rgba([128, 0, 0, 255])
  }
  fn orange_red() -> Self {
    Rgba([255, 69, 0, 255])
  }
  fn tomato() -> Self {
    Rgba([255, 99, 71, 255])
  }
  fn coral() -> Self {
    Rgba([255, 127, 80, 255])
  }
  fn dark_orange() -> Self {
    Rgba([255, 140, 0, 255])
  }
  fn orange() -> Self {
    Rgba([255, 165, 0, 255])
  }
  fn light_yellow() -> Self {
    Rgba([255, 255, 224, 255])
  }
  fn lemon_chiffon() -> Self {
    Rgba([255, 250, 205, 255])
  }
  fn light_goldenrod_yellow() -> Self {
    Rgba([250, 250, 210, 255])
  }
  fn papaya_whip() -> Self {
    Rgba([255, 239, 213, 255])
  }
  fn moccasin() -> Self {
    Rgba([255, 228, 181, 255])
  }
  fn peach_puff() -> Self {
    Rgba([255, 218, 185, 255])
  }
  fn pale_goldenrod() -> Self {
    Rgba([238, 232, 170, 255])
  }
  fn khaki() -> Self {
    Rgba([240, 230, 140, 255])
  }
  fn dark_khaki() -> Self {
    Rgba([189, 183, 107, 255])
  }
  fn gold() -> Self {
    Rgba([255, 215, 0, 255])
  }
  fn cornsilk() -> Self {
    Rgba([255, 248, 220, 255])
  }
  fn blanched_almond() -> Self {
    Rgba([255, 235, 205, 255])
  }
  fn bisque() -> Self {
    Rgba([255, 228, 196, 255])
  }
  fn navajo_white() -> Self {
    Rgba([255, 222, 173, 255])
  }
  fn wheat() -> Self {
    Rgba([245, 222, 179, 255])
  }
  fn burly_wood() -> Self {
    Rgba([222, 184, 135, 255])
  }
  fn tan() -> Self {
    Rgba([210, 180, 140, 255])
  }
  fn rosy_brown() -> Self {
    Rgba([188, 143, 143, 255])
  }
  fn sandy_brown() -> Self {
    Rgba([244, 164, 96, 255])
  }
  fn goldenrod() -> Self {
    Rgba([218, 165, 32, 255])
  }
  fn dark_goldenrod() -> Self {
    Rgba([184, 134, 11, 255])
  }
  fn peru() -> Self {
    Rgba([205, 133, 63, 255])
  }
  fn chocolate() -> Self {
    Rgba([210, 105, 30, 255])
  }
  fn saddle_brown() -> Self {
    Rgba([139, 69, 19, 255])
  }
  fn sienna() -> Self {
    Rgba([160, 82, 45, 255])
  }
  fn brown() -> Self {
    Rgba([165, 42, 42, 255])
  }
  fn dark_olive_green() -> Self {
    Rgba([85, 107, 47, 255])
  }
  fn olive() -> Self {
    Rgba([128, 128, 0, 255])
  }
  fn olive_drab() -> Self {
    Rgba([107, 142, 35, 255])
  }
  fn yellow_green() -> Self {
    Rgba([154, 205, 50, 255])
  }
  fn lime_green() -> Self {
    Rgba([50, 205, 50, 255])
  }
  fn lime() -> Self {
    Rgba([0, 255, 0, 255])
  }
  fn lawn_green() -> Self {
    Rgba([124, 252, 0, 255])
  }
  fn chartreuse() -> Self {
    Rgba([127, 255, 0, 255])
  }
  fn green_yellow() -> Self {
    Rgba([173, 255, 47, 255])
  }
  fn spring_green() -> Self {
    Rgba([0, 255, 127, 255])
  }
  fn medium_spring_green() -> Self {
    Rgba([0, 250, 154, 255])
  }
  fn light_green() -> Self {
    Rgba([144, 238, 144, 255])
  }
  fn pale_green() -> Self {
    Rgba([152, 251, 152, 255])
  }
  fn dark_sea_green() -> Self {
    Rgba([143, 188, 143, 255])
  }
  fn medium_sea_green() -> Self {
    Rgba([60, 179, 113, 255])
  }
  fn sea_green() -> Self {
    Rgba([46, 139, 87, 255])
  }
  fn forest_green() -> Self {
    Rgba([34, 139, 34, 255])
  }
  fn dark_green() -> Self {
    Rgba([0, 100, 0, 255])
  }
  fn medium_aquamarine() -> Self {
    Rgba([102, 205, 170, 255])
  }
  fn aquamarine() -> Self {
    Rgba([127, 255, 212, 255])
  }
  fn light_cyan() -> Self {
    Rgba([224, 255, 255, 255])
  }
  fn cyan() -> Self {
    Rgba([0, 255, 255, 255])
  }
  fn aqua() -> Self {
    Rgba([0, 255, 255, 255])
  }
  fn pale_turquoise() -> Self {
    Rgba([175, 238, 238, 255])
  }
  fn turquoise() -> Self {
    Rgba([64, 224, 208, 255])
  }
  fn medium_turquoise() -> Self {
    Rgba([72, 209, 204, 255])
  }
  fn dark_turquoise() -> Self {
    Rgba([0, 206, 209, 255])
  }
  fn light_sea_green() -> Self {
    Rgba([32, 178, 170, 255])
  }
  fn cadet_blue() -> Self {
    Rgba([95, 158, 160, 255])
  }
  fn dark_cyan() -> Self {
    Rgba([0, 139, 139, 255])
  }
  fn teal() -> Self {
    Rgba([0, 128, 128, 255])
  }
  fn light_steel_blue() -> Self {
    Rgba([176, 196, 222, 255])
  }
  fn powder_blue() -> Self {
    Rgba([176, 224, 230, 255])
  }
  fn light_blue() -> Self {
    Rgba([173, 216, 230, 255])
  }
  fn sky_blue() -> Self {
    Rgba([135, 206, 235, 255])
  }
  fn light_sky_blue() -> Self {
    Rgba([135, 206, 250, 255])
  }
  fn deep_sky_blue() -> Self {
    Rgba([0, 191, 255, 255])
  }
  fn dodger_blue() -> Self {
    Rgba([30, 144, 255, 255])
  }
  fn cornflower_blue() -> Self {
    Rgba([100, 149, 237, 255])
  }
  fn steel_blue() -> Self {
    Rgba([70, 130, 180, 255])
  }
  fn royal_blue() -> Self {
    Rgba([65, 105, 225, 255])
  }
  fn medium_blue() -> Self {
    Rgba([0, 0, 205, 255])
  }
  fn dark_blue() -> Self {
    Rgba([0, 0, 139, 255])
  }
  fn navy() -> Self {
    Rgba([0, 0, 128, 255])
  }
  fn midnight_blue() -> Self {
    Rgba([25, 25, 112, 255])
  }
  fn medium_slate_blue() -> Self {
    Rgba([123, 104, 238, 255])
  }
  fn slate_blue() -> Self {
    Rgba([106, 90, 205, 255])
  }
  fn dark_slate_blue() -> Self {
    Rgba([72, 61, 139, 255])
  }
  fn lavender() -> Self {
    Rgba([230, 230, 250, 255])
  }
  fn thistle() -> Self {
    Rgba([216, 191, 216, 255])
  }
  fn plum() -> Self {
    Rgba([221, 160, 221, 255])
  }
  fn violet() -> Self {
    Rgba([238, 130, 238, 255])
  }
  fn orchid() -> Self {
    Rgba([218, 112, 214, 255])
  }
  fn fuchsia() -> Self {
    Rgba([255, 0, 255, 255])
  }
  fn magenta() -> Self {
    Rgba([255, 0, 255, 255])
  }
  fn medium_orchid() -> Self {
    Rgba([186, 85, 211, 255])
  }
  fn medium_purple() -> Self {
    Rgba([147, 112, 219, 255])
  }
  fn blue_violet() -> Self {
    Rgba([138, 43, 226, 255])
  }
  fn dark_violet() -> Self {
    Rgba([148, 0, 211, 255])
  }
  fn dark_orchid() -> Self {
    Rgba([153, 50, 204, 255])
  }
  fn dark_magenta() -> Self {
    Rgba([139, 0, 139, 255])
  }
  fn purple() -> Self {
    Rgba([128, 0, 128, 255])
  }
  fn indigo() -> Self {
    Rgba([75, 0, 130, 255])
  }
  fn dark_slate_gray() -> Self {
    Rgba([47, 79, 79, 255])
  }
  fn white_smoke() -> Self {
    Rgba([245, 245, 245, 255])
  }
  fn honeydew() -> Self {
    Rgba([240, 255, 240, 255])
  }
  fn mint_cream() -> Self {
    Rgba([245, 255, 250, 255])
  }
  fn azure() -> Self {
    Rgba([240, 255, 255, 255])
  }
  fn alice_blue() -> Self {
    Rgba([240, 248, 255, 255])
  }
  fn ghost_white() -> Self {
    Rgba([248, 248, 255, 255])
  }
  fn sea_shell() -> Self {
    Rgba([255, 245, 238, 255])
  }
  fn beige() -> Self {
    Rgba([245, 245, 220, 255])
  }
  fn old_lace() -> Self {
    Rgba([253, 245, 230, 255])
  }
  fn floral_white() -> Self {
    Rgba([255, 250, 240, 255])
  }
  fn ivory() -> Self {
    Rgba([255, 255, 240, 255])
  }
  fn antique_white() -> Self {
    Rgba([250, 235, 215, 255])
  }
  fn linen() -> Self {
    Rgba([250, 240, 230, 255])
  }
  fn lavender_blush() -> Self {
    Rgba([255, 240, 245, 255])
  }
  fn misty_rose() -> Self {
    Rgba([255, 228, 225, 255])
  }
  fn gainsboro() -> Self {
    Rgba([220, 220, 220, 255])
  }
  fn light_gray() -> Self {
    Rgba([211, 211, 211, 255])
  }
  fn silver() -> Self {
    Rgba([192, 192, 192, 255])
  }
  fn dark_gray() -> Self {
    Rgba([169, 169, 169, 255])
  }
  fn gray() -> Self {
    Rgba([128, 128, 128, 255])
  }
  fn dim_gray() -> Self {
    Rgba([105, 105, 105, 255])
  }
  fn light_slate_gray() -> Self {
    Rgba([119, 136, 153, 255])
  }
  fn slate_gray() -> Self {
    Rgba([112, 128, 144, 255])
  }
  fn dark_slate_grey() -> Self {
    Rgba([47, 79, 79, 255])
  }
  fn charcoal() -> Self {
    Rgba([54, 69, 79, 255])
  }
  fn rebecca_purple() -> Self {
    Rgba([102, 51, 153, 255])
  }
  fn off() -> Self {
    Rgba([0, 0, 0, 255])
  }
  fn default() -> Self {
    Rgba([0, 0, 0, 0])
  }
}

pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
  let r = r as f32 / 255.0;
  let g = g as f32 / 255.0;
  let b = b as f32 / 255.0;

  let max = r.max(g).max(b);
  let min = r.min(g).min(b);
  let delta = max - min;

  let h = if delta == 0.0 {
    0.0
  } else if max == r {
    60.0 * (((g - b) / delta).rem_euclid(6.0))
  } else if max == g {
    60.0 * (((b - r) / delta) + 2.0)
  } else {
    60.0 * (((r - g) / delta) + 4.0)
  };

  let s = if max == 0.0 { 0.0 } else { delta / max };
  let v = max;

  (h, s, v)
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
  let c = v * s;
  let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
  let m = v - c;

  let (r1, g1, b1) = match h as u32 {
    0..=59 => (c, x, 0.0),
    60..=119 => (x, c, 0.0),
    120..=179 => (0.0, c, x),
    180..=239 => (0.0, x, c),
    240..=299 => (x, 0.0, c),
    _ => (c, 0.0, x),
  };

  (
    ((r1 + m) * 255.0).round() as u8,
    ((g1 + m) * 255.0).round() as u8,
    ((b1 + m) * 255.0).round() as u8,
  )
}

fn to_hsl(c: Rgba<u8>) -> (Hsl, u8) {
  let srgb: Srgb<u8> = Srgb::new(c.0[0], c.0[1], c.0[2]);
  let hsl: Hsl = srgb.into_format::<f32>().into_color();
  (hsl, c.0[3])
}

fn from_hsl(hsl: Hsl, alpha: u8) -> Rgba<u8> {
  let srgb: Srgb<f32> = Srgb::from_color(hsl);
  let srgb_u8 = srgb.into_format::<u8>();
  Rgba([srgb_u8.red, srgb_u8.green, srgb_u8.blue, alpha])
}
