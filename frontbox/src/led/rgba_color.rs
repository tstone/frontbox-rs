use fast_protocol::Color;
use image::Rgba;
use palette::{FromColor, Hsl, IntoColor, Srgb};

use crate::prelude::LedChannels;

pub trait RgbaColor {
  fn red() -> Self;
  fn yellow() -> Self;
  fn blue() -> Self;
  fn green() -> Self;
  fn white() -> Self;
  fn black() -> Self;
  fn pink() -> Self;
  fn purple() -> Self;
  fn magenta() -> Self;
  fn cyan() -> Self;
  fn turquoise() -> Self;
  fn lime() -> Self;
  fn orange() -> Self;
  fn transparent() -> Self;

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
  fn with_luma(&self) -> f32;
  fn hue_shift(self, degrees: f32) -> Self;
  fn lighten(self, amount: f32) -> Self;
  fn darken(self, amount: f32) -> Self;
  fn saturate(self, amount: f32) -> Self;
  fn desaturate(self, amount: f32) -> Self;
  fn inverted(self) -> Self;

  fn brightness(&self) -> f32;

  fn over(src: Self, dst: Self) -> Self;
  fn composite_over(&self, dst: Self) -> Self;
}

impl RgbaColor for Rgba<u8> {
  fn with_luma(&self) -> f32 {
    (0.2126 * self.0[0] as f32 + 0.7152 * self.0[1] as f32 + 0.0722 * self.0[2] as f32) / 255.0
  }

  fn brightness(&self) -> f32 {
    (0.299 * self[0] as f32 + 0.587 * self[1] as f32 + 0.114 * self[2] as f32) / 255.0
  }

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

  fn saturate(self, amount: f32) -> Self {
    let (mut hsl, a) = to_hsl(self);
    hsl.saturation = (hsl.saturation + amount).clamp(0.0, 1.0);
    from_hsl(hsl, a)
  }

  fn desaturate(self, amount: f32) -> Self {
    let (mut hsl, a) = to_hsl(self);
    hsl.saturation = (hsl.saturation - amount).clamp(0.0, 1.0);
    from_hsl(hsl, a)
  }

  fn with_lightness(self, value: f32) -> Self {
    let (mut hsl, a) = to_hsl(self);
    hsl.lightness = value.clamp(0.0, 1.0);
    from_hsl(hsl, a)
  }

  fn darken(self, amount: f32) -> Self {
    let (mut hsl, a) = to_hsl(self);
    hsl.lightness = (hsl.lightness - amount).clamp(0.0, 1.0);
    from_hsl(hsl, a)
  }

  fn lighten(self, amount: f32) -> Self {
    let (mut hsl, a) = to_hsl(self);
    hsl.lightness = (hsl.lightness + amount).clamp(0.0, 1.0);
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

  fn hue_shift(self, degrees: f32) -> Self {
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
  fn purple() -> Self {
    Rgba([127, 0, 225, 255])
  }
  fn cyan() -> Self {
    Rgba([0, 255, 255, 255])
  }
  fn turquoise() -> Self {
    Rgba([64, 224, 208, 255])
  }
  fn magenta() -> Self {
    Rgba([255, 0, 255, 255])
  }
  fn lime() -> Self {
    Rgba([50, 205, 50, 255])
  }
  fn orange() -> Self {
    Rgba([255, 165, 0, 255])
  }

  fn default() -> Self {
    Rgba([0, 0, 0, 0])
  }

  fn off() -> Self {
    Self::black()
  }
  fn transparent() -> Self {
    Self::default()
  }
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
