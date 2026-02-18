use crate::prelude::Lerp;

#[derive(Debug, Clone, PartialEq)]
pub struct Color {
  pub r: f32,
  pub g: f32,
  pub b: f32,
  pub w: Option<f32>,
}

impl Color {
  pub fn rgbw(r: f32, g: f32, b: f32, w: f32) -> Self {
    Self {
      r,
      g,
      b,
      w: Some(w),
    }
  }

  pub fn rgb(r: f32, g: f32, b: f32) -> Self {
    Self { r, g, b, w: None }
  }

  pub fn mix(&self, other: &Self, t: f32) -> Self {
    let r = self.r + (other.r - self.r) * t;
    let g = self.g + (other.g - self.g) * t;
    let b = self.b + (other.b - self.b) * t;

    let w = match (self.w, other.w) {
      (Some(w1), Some(w2)) => Some(w1 + (w2 - w1) * t),
      _ => None,
    };

    Self { r, g, b, w }
  }

  pub fn to_hex(&self) -> String {
    let r = (self.r.clamp(0.0, 1.0) * 255.0) as u8;
    let g = (self.g.clamp(0.0, 1.0) * 255.0) as u8;
    let b = (self.b.clamp(0.0, 1.0) * 255.0) as u8;

    if let Some(w) = self.w {
      let w = (w.clamp(0.0, 1.0) * 255.0) as u8;
      format!("{:02X}{:02X}{:02X}{:02X}", r, g, b, w)
    } else {
      format!("{:02X}{:02X}{:02X}", r, g, b)
    }
  }

  pub fn red() -> Self {
    Self::rgb(1.0, 0.0, 0.0)
  }

  pub fn green() -> Self {
    Self::rgb(0.0, 1.0, 0.0)
  }

  pub fn blue() -> Self {
    Self::rgb(0.0, 0.0, 1.0)
  }

  pub fn white() -> Self {
    Self::rgbw(0.0, 0.0, 0.0, 1.0)
  }

  pub fn white_rgb() -> Self {
    Self::rgbw(1.0, 1.0, 1.0, 1.0)
  }

  pub fn black() -> Self {
    Self::rgb(0.0, 0.0, 0.0)
  }

  pub fn yellow() -> Self {
    Self::rgb(1.0, 1.0, 0.0)
  }

  // CSS Named Colors — Pink tones
  pub fn pink() -> Self {
    Self::rgb(255.0 / 255.0, 192.0 / 255.0, 203.0 / 255.0)
  }
  pub fn light_pink() -> Self {
    Self::rgb(255.0 / 255.0, 182.0 / 255.0, 193.0 / 255.0)
  }
  pub fn hot_pink() -> Self {
    Self::rgb(255.0 / 255.0, 105.0 / 255.0, 180.0 / 255.0)
  }
  pub fn deep_pink() -> Self {
    Self::rgb(255.0 / 255.0, 20.0 / 255.0, 147.0 / 255.0)
  }
  pub fn pale_violet_red() -> Self {
    Self::rgb(219.0 / 255.0, 112.0 / 255.0, 147.0 / 255.0)
  }
  pub fn medium_violet_red() -> Self {
    Self::rgb(199.0 / 255.0, 21.0 / 255.0, 133.0 / 255.0)
  }

  // CSS Named Colors — Red tones
  pub fn light_salmon() -> Self {
    Self::rgb(255.0 / 255.0, 160.0 / 255.0, 122.0 / 255.0)
  }
  pub fn salmon() -> Self {
    Self::rgb(250.0 / 255.0, 128.0 / 255.0, 114.0 / 255.0)
  }
  pub fn dark_salmon() -> Self {
    Self::rgb(233.0 / 255.0, 150.0 / 255.0, 122.0 / 255.0)
  }
  pub fn light_coral() -> Self {
    Self::rgb(240.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0)
  }
  pub fn indian_red() -> Self {
    Self::rgb(205.0 / 255.0, 92.0 / 255.0, 92.0 / 255.0)
  }
  pub fn crimson() -> Self {
    Self::rgb(220.0 / 255.0, 20.0 / 255.0, 60.0 / 255.0)
  }
  pub fn firebrick() -> Self {
    Self::rgb(178.0 / 255.0, 34.0 / 255.0, 34.0 / 255.0)
  }
  pub fn dark_red() -> Self {
    Self::rgb(139.0 / 255.0, 0.0, 0.0)
  }
  pub fn maroon() -> Self {
    Self::rgb(128.0 / 255.0, 0.0, 0.0)
  }

  // CSS Named Colors — Orange tones
  pub fn orange_red() -> Self {
    Self::rgb(255.0 / 255.0, 69.0 / 255.0, 0.0)
  }
  pub fn tomato() -> Self {
    Self::rgb(255.0 / 255.0, 99.0 / 255.0, 71.0 / 255.0)
  }
  pub fn coral() -> Self {
    Self::rgb(255.0 / 255.0, 127.0 / 255.0, 80.0 / 255.0)
  }
  pub fn dark_orange() -> Self {
    Self::rgb(255.0 / 255.0, 140.0 / 255.0, 0.0)
  }
  pub fn orange() -> Self {
    Self::rgb(255.0 / 255.0, 165.0 / 255.0, 0.0)
  }

  // CSS Named Colors — Yellow tones
  pub fn light_yellow() -> Self {
    Self::rgb(255.0 / 255.0, 255.0 / 255.0, 224.0 / 255.0)
  }
  pub fn lemon_chiffon() -> Self {
    Self::rgb(255.0 / 255.0, 250.0 / 255.0, 205.0 / 255.0)
  }
  pub fn light_goldenrod_yellow() -> Self {
    Self::rgb(250.0 / 255.0, 250.0 / 255.0, 210.0 / 255.0)
  }
  pub fn papaya_whip() -> Self {
    Self::rgb(255.0 / 255.0, 239.0 / 255.0, 213.0 / 255.0)
  }
  pub fn moccasin() -> Self {
    Self::rgb(255.0 / 255.0, 228.0 / 255.0, 181.0 / 255.0)
  }
  pub fn peach_puff() -> Self {
    Self::rgb(255.0 / 255.0, 218.0 / 255.0, 185.0 / 255.0)
  }
  pub fn pale_goldenrod() -> Self {
    Self::rgb(238.0 / 255.0, 232.0 / 255.0, 170.0 / 255.0)
  }
  pub fn khaki() -> Self {
    Self::rgb(240.0 / 255.0, 230.0 / 255.0, 140.0 / 255.0)
  }
  pub fn dark_khaki() -> Self {
    Self::rgb(189.0 / 255.0, 183.0 / 255.0, 107.0 / 255.0)
  }
  pub fn gold() -> Self {
    Self::rgb(255.0 / 255.0, 215.0 / 255.0, 0.0)
  }

  // CSS Named Colors — Brown tones
  pub fn cornsilk() -> Self {
    Self::rgb(255.0 / 255.0, 248.0 / 255.0, 220.0 / 255.0)
  }
  pub fn blanched_almond() -> Self {
    Self::rgb(255.0 / 255.0, 235.0 / 255.0, 205.0 / 255.0)
  }
  pub fn bisque() -> Self {
    Self::rgb(255.0 / 255.0, 228.0 / 255.0, 196.0 / 255.0)
  }
  pub fn navajo_white() -> Self {
    Self::rgb(255.0 / 255.0, 222.0 / 255.0, 173.0 / 255.0)
  }
  pub fn wheat() -> Self {
    Self::rgb(245.0 / 255.0, 222.0 / 255.0, 179.0 / 255.0)
  }
  pub fn burly_wood() -> Self {
    Self::rgb(222.0 / 255.0, 184.0 / 255.0, 135.0 / 255.0)
  }
  pub fn tan() -> Self {
    Self::rgb(210.0 / 255.0, 180.0 / 255.0, 140.0 / 255.0)
  }
  pub fn rosy_brown() -> Self {
    Self::rgb(188.0 / 255.0, 143.0 / 255.0, 143.0 / 255.0)
  }
  pub fn sandy_brown() -> Self {
    Self::rgb(244.0 / 255.0, 164.0 / 255.0, 96.0 / 255.0)
  }
  pub fn goldenrod() -> Self {
    Self::rgb(218.0 / 255.0, 165.0 / 255.0, 32.0 / 255.0)
  }
  pub fn dark_goldenrod() -> Self {
    Self::rgb(184.0 / 255.0, 134.0 / 255.0, 11.0 / 255.0)
  }
  pub fn peru() -> Self {
    Self::rgb(205.0 / 255.0, 133.0 / 255.0, 63.0 / 255.0)
  }
  pub fn chocolate() -> Self {
    Self::rgb(210.0 / 255.0, 105.0 / 255.0, 30.0 / 255.0)
  }
  pub fn saddle_brown() -> Self {
    Self::rgb(139.0 / 255.0, 69.0 / 255.0, 19.0 / 255.0)
  }
  pub fn sienna() -> Self {
    Self::rgb(160.0 / 255.0, 82.0 / 255.0, 45.0 / 255.0)
  }
  pub fn brown() -> Self {
    Self::rgb(165.0 / 255.0, 42.0 / 255.0, 42.0 / 255.0)
  }

  // CSS Named Colors — Green tones
  pub fn dark_olive_green() -> Self {
    Self::rgb(85.0 / 255.0, 107.0 / 255.0, 47.0 / 255.0)
  }
  pub fn olive() -> Self {
    Self::rgb(128.0 / 255.0, 128.0 / 255.0, 0.0)
  }
  pub fn olive_drab() -> Self {
    Self::rgb(107.0 / 255.0, 142.0 / 255.0, 35.0 / 255.0)
  }
  pub fn yellow_green() -> Self {
    Self::rgb(154.0 / 255.0, 205.0 / 255.0, 50.0 / 255.0)
  }
  pub fn lime_green() -> Self {
    Self::rgb(50.0 / 255.0, 205.0 / 255.0, 50.0 / 255.0)
  }
  pub fn lime() -> Self {
    Self::rgb(0.0, 1.0, 0.0)
  }
  pub fn lawn_green() -> Self {
    Self::rgb(124.0 / 255.0, 252.0 / 255.0, 0.0)
  }
  pub fn chartreuse() -> Self {
    Self::rgb(127.0 / 255.0, 255.0 / 255.0, 0.0)
  }
  pub fn green_yellow() -> Self {
    Self::rgb(173.0 / 255.0, 255.0 / 255.0, 47.0 / 255.0)
  }
  pub fn spring_green() -> Self {
    Self::rgb(0.0, 255.0 / 255.0, 127.0 / 255.0)
  }
  pub fn medium_spring_green() -> Self {
    Self::rgb(0.0, 250.0 / 255.0, 154.0 / 255.0)
  }
  pub fn light_green() -> Self {
    Self::rgb(144.0 / 255.0, 238.0 / 255.0, 144.0 / 255.0)
  }
  pub fn pale_green() -> Self {
    Self::rgb(152.0 / 255.0, 251.0 / 255.0, 152.0 / 255.0)
  }
  pub fn dark_sea_green() -> Self {
    Self::rgb(143.0 / 255.0, 188.0 / 255.0, 143.0 / 255.0)
  }
  pub fn medium_sea_green() -> Self {
    Self::rgb(60.0 / 255.0, 179.0 / 255.0, 113.0 / 255.0)
  }
  pub fn sea_green() -> Self {
    Self::rgb(46.0 / 255.0, 139.0 / 255.0, 87.0 / 255.0)
  }
  pub fn forest_green() -> Self {
    Self::rgb(34.0 / 255.0, 139.0 / 255.0, 34.0 / 255.0)
  }
  pub fn dark_green() -> Self {
    Self::rgb(0.0, 100.0 / 255.0, 0.0)
  }
  pub fn medium_aquamarine() -> Self {
    Self::rgb(102.0 / 255.0, 205.0 / 255.0, 170.0 / 255.0)
  }
  pub fn aquamarine() -> Self {
    Self::rgb(127.0 / 255.0, 255.0 / 255.0, 212.0 / 255.0)
  }

  // CSS Named Colors — Cyan tones
  pub fn light_cyan() -> Self {
    Self::rgb(224.0 / 255.0, 255.0 / 255.0, 255.0 / 255.0)
  }
  pub fn cyan() -> Self {
    Self::rgb(0.0, 1.0, 1.0)
  }
  pub fn aqua() -> Self {
    Self::rgb(0.0, 1.0, 1.0)
  }
  pub fn pale_turquoise() -> Self {
    Self::rgb(175.0 / 255.0, 238.0 / 255.0, 238.0 / 255.0)
  }
  pub fn turquoise() -> Self {
    Self::rgb(64.0 / 255.0, 224.0 / 255.0, 208.0 / 255.0)
  }
  pub fn medium_turquoise() -> Self {
    Self::rgb(72.0 / 255.0, 209.0 / 255.0, 204.0 / 255.0)
  }
  pub fn dark_turquoise() -> Self {
    Self::rgb(0.0, 206.0 / 255.0, 209.0 / 255.0)
  }
  pub fn light_sea_green() -> Self {
    Self::rgb(32.0 / 255.0, 178.0 / 255.0, 170.0 / 255.0)
  }
  pub fn cadet_blue() -> Self {
    Self::rgb(95.0 / 255.0, 158.0 / 255.0, 160.0 / 255.0)
  }
  pub fn dark_cyan() -> Self {
    Self::rgb(0.0, 139.0 / 255.0, 139.0 / 255.0)
  }
  pub fn teal() -> Self {
    Self::rgb(0.0, 128.0 / 255.0, 128.0 / 255.0)
  }

  // CSS Named Colors — Blue tones
  pub fn light_steel_blue() -> Self {
    Self::rgb(176.0 / 255.0, 196.0 / 255.0, 222.0 / 255.0)
  }
  pub fn powder_blue() -> Self {
    Self::rgb(176.0 / 255.0, 224.0 / 255.0, 230.0 / 255.0)
  }
  pub fn light_blue() -> Self {
    Self::rgb(173.0 / 255.0, 216.0 / 255.0, 230.0 / 255.0)
  }
  pub fn sky_blue() -> Self {
    Self::rgb(135.0 / 255.0, 206.0 / 255.0, 235.0 / 255.0)
  }
  pub fn light_sky_blue() -> Self {
    Self::rgb(135.0 / 255.0, 206.0 / 255.0, 250.0 / 255.0)
  }
  pub fn deep_sky_blue() -> Self {
    Self::rgb(0.0, 191.0 / 255.0, 255.0 / 255.0)
  }
  pub fn dodger_blue() -> Self {
    Self::rgb(30.0 / 255.0, 144.0 / 255.0, 255.0 / 255.0)
  }
  pub fn cornflower_blue() -> Self {
    Self::rgb(100.0 / 255.0, 149.0 / 255.0, 237.0 / 255.0)
  }
  pub fn steel_blue() -> Self {
    Self::rgb(70.0 / 255.0, 130.0 / 255.0, 180.0 / 255.0)
  }
  pub fn royal_blue() -> Self {
    Self::rgb(65.0 / 255.0, 105.0 / 255.0, 225.0 / 255.0)
  }
  pub fn medium_blue() -> Self {
    Self::rgb(0.0, 0.0, 205.0 / 255.0)
  }
  pub fn dark_blue() -> Self {
    Self::rgb(0.0, 0.0, 139.0 / 255.0)
  }
  pub fn navy() -> Self {
    Self::rgb(0.0, 0.0, 128.0 / 255.0)
  }
  pub fn midnight_blue() -> Self {
    Self::rgb(25.0 / 255.0, 25.0 / 255.0, 112.0 / 255.0)
  }
  pub fn medium_slate_blue() -> Self {
    Self::rgb(123.0 / 255.0, 104.0 / 255.0, 238.0 / 255.0)
  }
  pub fn slate_blue() -> Self {
    Self::rgb(106.0 / 255.0, 90.0 / 255.0, 205.0 / 255.0)
  }
  pub fn dark_slate_blue() -> Self {
    Self::rgb(72.0 / 255.0, 61.0 / 255.0, 139.0 / 255.0)
  }

  // CSS Named Colors — Purple/Violet tones
  pub fn lavender() -> Self {
    Self::rgb(230.0 / 255.0, 230.0 / 255.0, 250.0 / 255.0)
  }
  pub fn thistle() -> Self {
    Self::rgb(216.0 / 255.0, 191.0 / 255.0, 216.0 / 255.0)
  }
  pub fn plum() -> Self {
    Self::rgb(221.0 / 255.0, 160.0 / 255.0, 221.0 / 255.0)
  }
  pub fn violet() -> Self {
    Self::rgb(238.0 / 255.0, 130.0 / 255.0, 238.0 / 255.0)
  }
  pub fn orchid() -> Self {
    Self::rgb(218.0 / 255.0, 112.0 / 255.0, 214.0 / 255.0)
  }
  pub fn fuchsia() -> Self {
    Self::rgb(1.0, 0.0, 1.0)
  }
  pub fn magenta() -> Self {
    Self::rgb(1.0, 0.0, 1.0)
  }
  pub fn medium_orchid() -> Self {
    Self::rgb(186.0 / 255.0, 85.0 / 255.0, 211.0 / 255.0)
  }
  pub fn medium_purple() -> Self {
    Self::rgb(147.0 / 255.0, 112.0 / 255.0, 219.0 / 255.0)
  }
  pub fn blue_violet() -> Self {
    Self::rgb(138.0 / 255.0, 43.0 / 255.0, 226.0 / 255.0)
  }
  pub fn dark_violet() -> Self {
    Self::rgb(148.0 / 255.0, 0.0, 211.0 / 255.0)
  }
  pub fn dark_orchid() -> Self {
    Self::rgb(153.0 / 255.0, 50.0 / 255.0, 204.0 / 255.0)
  }
  pub fn dark_magenta() -> Self {
    Self::rgb(139.0 / 255.0, 0.0, 139.0 / 255.0)
  }
  pub fn purple() -> Self {
    Self::rgb(128.0 / 255.0, 0.0, 128.0 / 255.0)
  }
  pub fn indigo() -> Self {
    Self::rgb(75.0 / 255.0, 0.0, 130.0 / 255.0)
  }
  pub fn dark_slate_gray() -> Self {
    Self::rgb(47.0 / 255.0, 79.0 / 255.0, 79.0 / 255.0)
  }

  // CSS Named Colors — White tones
  pub fn white_smoke() -> Self {
    Self::rgb(245.0 / 255.0, 245.0 / 255.0, 245.0 / 255.0)
  }
  pub fn honeydew() -> Self {
    Self::rgb(240.0 / 255.0, 255.0 / 255.0, 240.0 / 255.0)
  }
  pub fn mint_cream() -> Self {
    Self::rgb(245.0 / 255.0, 255.0 / 255.0, 250.0 / 255.0)
  }
  pub fn azure() -> Self {
    Self::rgb(240.0 / 255.0, 255.0 / 255.0, 255.0 / 255.0)
  }
  pub fn alice_blue() -> Self {
    Self::rgb(240.0 / 255.0, 248.0 / 255.0, 255.0 / 255.0)
  }
  pub fn ghost_white() -> Self {
    Self::rgb(248.0 / 255.0, 248.0 / 255.0, 255.0 / 255.0)
  }
  pub fn sea_shell() -> Self {
    Self::rgb(255.0 / 255.0, 245.0 / 255.0, 238.0 / 255.0)
  }
  pub fn beige() -> Self {
    Self::rgb(245.0 / 255.0, 245.0 / 255.0, 220.0 / 255.0)
  }
  pub fn old_lace() -> Self {
    Self::rgb(253.0 / 255.0, 245.0 / 255.0, 230.0 / 255.0)
  }
  pub fn floral_white() -> Self {
    Self::rgb(255.0 / 255.0, 250.0 / 255.0, 240.0 / 255.0)
  }
  pub fn ivory() -> Self {
    Self::rgb(255.0 / 255.0, 255.0 / 255.0, 240.0 / 255.0)
  }
  pub fn antique_white() -> Self {
    Self::rgb(250.0 / 255.0, 235.0 / 255.0, 215.0 / 255.0)
  }
  pub fn linen() -> Self {
    Self::rgb(250.0 / 255.0, 240.0 / 255.0, 230.0 / 255.0)
  }
  pub fn lavender_blush() -> Self {
    Self::rgb(255.0 / 255.0, 240.0 / 255.0, 245.0 / 255.0)
  }
  pub fn misty_rose() -> Self {
    Self::rgb(255.0 / 255.0, 228.0 / 255.0, 225.0 / 255.0)
  }

  // CSS Named Colors — Gray/Black tones
  pub fn gainsboro() -> Self {
    Self::rgb(220.0 / 255.0, 220.0 / 255.0, 220.0 / 255.0)
  }
  pub fn light_gray() -> Self {
    Self::rgb(211.0 / 255.0, 211.0 / 255.0, 211.0 / 255.0)
  }
  pub fn silver() -> Self {
    Self::rgb(192.0 / 255.0, 192.0 / 255.0, 192.0 / 255.0)
  }
  pub fn dark_gray() -> Self {
    Self::rgb(169.0 / 255.0, 169.0 / 255.0, 169.0 / 255.0)
  }
  pub fn gray() -> Self {
    Self::rgb(128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0)
  }
  pub fn dim_gray() -> Self {
    Self::rgb(105.0 / 255.0, 105.0 / 255.0, 105.0 / 255.0)
  }
  pub fn light_slate_gray() -> Self {
    Self::rgb(119.0 / 255.0, 136.0 / 255.0, 153.0 / 255.0)
  }
  pub fn slate_gray() -> Self {
    Self::rgb(112.0 / 255.0, 128.0 / 255.0, 144.0 / 255.0)
  }
  pub fn dark_slate_grey() -> Self {
    Self::rgb(47.0 / 255.0, 79.0 / 255.0, 79.0 / 255.0)
  }
  pub fn charcoal() -> Self {
    Self::rgb(54.0 / 255.0, 69.0 / 255.0, 79.0 / 255.0)
  }
  pub fn rebecca_purple() -> Self {
    Self::rgb(102.0 / 255.0, 51.0 / 255.0, 153.0 / 255.0)
  }
}

impl Lerp for Color {
  fn interpolate(&self, other: &Self, t: f32) -> Self {
    self.mix(other, t)
  }
}

impl Default for Color {
  fn default() -> Self {
    Self::black()
  }
}
